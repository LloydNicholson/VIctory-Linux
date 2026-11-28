//! Translated shaders kept on disk (decision 0003): under the configured
//! cache folder, keyed by the container's content, by the translator's
//! version ([`victory_shader::TRANSLATOR_VERSION`]) and by its settings.
//! A rebuild that leaves the version keeps every translation: the store
//! grows by the shaders a change made translate, not by a whole new set
//! each build (16 GB of sets of earlier builds had piled up when the key
//! was the translator's source). Entries go through `victory-pipeline`'s
//! content-addressed cache, which writes them atomically; an entry that
//! does not read back is translated again.
//!
//! Each entry also names the build of the translator that made it (the
//! BLAKE3 of its sources, `build.rs`). One of an earlier build is served
//! as it is, and counted; `victory shaders` ([`precompile`]) translates it
//! again, keeps the new one and says how many came out different (then
//! the version should have been bumped). `VICTORY_SHADER_FRESH=1` does the
//! same in a run; `VICTORY_SHADER_KEY=source` keys by the translator's
//! source as before (a separate set for each build).
//!
//! The entry is our own encoding of [`HostShader`]: a field added to it
//! must be added to `encode` and `decode` too, and [`FORMAT`] bumped.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The encoding of entries ([`encode`]), part of the key.
const FORMAT: u32 = 2;

use victory_pipeline::{Cache, Converter};
use victory_shader::emit::{HostShader, StaticSampler, UserData};
use victory_shader::resource::RootIndexedView;
use victory_shader::{HwStage, RootType};

static CACHE: OnceLock<Cache> = OnceLock::new();
static HITS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);
/// Hits on entries an earlier build of the translator made (served as they
/// are, or when refreshing translated again); of those translated again,
/// how many came out different.
static STALE: AtomicU64 = AtomicU64::new(0);
static REFRESHED: AtomicU64 = AtomicU64::new(0);
static CHANGED: AtomicU64 = AtomicU64::new(0);

/// The build of the translator (the BLAKE3 of its sources, `build.rs`).
const SOURCE: &str = env!("VICTORY_TRANSLATOR");

/// How cached translations are keyed (`VICTORY_SHADER_KEY`).
fn keyed_by_source() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("VICTORY_SHADER_KEY").is_ok_and(|v| v == "source"))
}

/// Whether a run translates entries of an earlier build again
/// (`VICTORY_SHADER_FRESH=1`; [`precompile`] always does).
fn refresh_in_runs() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_SHADER_FRESH").is_some_and(|v| v == "1"))
}

/// Keeps translated shaders in `dir` from now on.
pub fn use_shader_cache(dir: PathBuf) {
    match Cache::open(dir) {
        Ok(cache) => {
            let _ = CACHE.set(cache);
        }
        Err(e) => tracing::warn!(error = %e, "no shader cache"),
    }
}

/// Shaders found in the cache and translated since the start.
pub(crate) fn counts() -> (u64, u64) {
    (HITS.load(Ordering::Relaxed), MISSES.load(Ordering::Relaxed))
}

/// Since the start: entries of an earlier build of the translator met,
/// those of them translated again, and those that came out different.
pub fn stale_counts() -> (u64, u64, u64) {
    (STALE.load(Ordering::Relaxed), REFRESHED.load(Ordering::Relaxed), CHANGED.load(Ordering::Relaxed))
}

/// What identifies a translation beside its container: the translator's
/// version (or with `VICTORY_SHADER_KEY=source` its sources), the form of
/// entries, the settings it runs with, and what it was told of the host.
/// Not our executable: a rebuild of anything else used to translate every
/// shader again.
fn build() -> [u8; 32] {
    // The view heap's null slot differs by host GPU: a translation cached
    // without the clamp must not be served (`host_settings` below).
    crate::descriptor::host::probe_view_limit();
    static FIXED: OnceLock<Vec<u8>> = OnceLock::new();
    let fixed = FIXED.get_or_init(|| {
        let limit = victory_shader::emit::loop_limit();
        if limit != victory_shader::emit::DEFAULT_LOOP_LIMIT {
            tracing::warn!(
                limit,
                "VICTORY_SHADER_LOOP_LIMIT: translated loops stop sooner"
            );
        }
        let mut bytes = if keyed_by_source() {
            SOURCE.as_bytes().to_vec()
        } else {
            format!("version {}", victory_shader::TRANSLATOR_VERSION).into_bytes()
        };
        bytes.extend_from_slice(&FORMAT.to_le_bytes());
        bytes.extend_from_slice(&limit.to_le_bytes());
        // Which group barriers wait is part of the translation too.
        bytes.push(victory_shader::emit::group_sync());
        // So are the switches that pick the form of a wave64 program:
        // without them here a run with `VICTORY_WAVE_WIDE=0` was
        // served the translations an earlier run of the same
        // executable had cached with the switch unset.
        for name in [
            "VICTORY_WAVE_WIDE",
            "VICTORY_WAVE_DEEP_SKIPS",
            "VICTORY_WAVE_OWN_BITS",
            "VICTORY_WAVE_LOCAL",
            "VICTORY_LDS_FLAGS",
            "VICTORY_F16_NEAREST",
            "VICTORY_HELPER_OWN",
            "VICTORY_FIRSTLANE_OWN",
            "VICTORY_FIRST_HALF",
            "VICTORY_STRICT_JOINS",
            "VICTORY_MESH_OUTPUT_STRICT",
            "VICTORY_BVH_FENCES",
            "VICTORY_BVH_BOUNDS",
            "VICTORY_HALF_VIEWS",
        ] {
            bytes.extend_from_slice(name.as_bytes());
            bytes.push(b'=');
            if let Some(value) = std::env::var_os(name) {
                bytes.extend_from_slice(value.as_encoded_bytes());
            }
            bytes.push(0);
        }
        // Switches added since, only when set: other runs keep their keys (an unset name in the
        // list above still changed every key: runs popimgcoh2 and cfcontrol started with 279 of
        // 3,861 shaders cached, a black picture at 1-3 fps).
        for name in ["VICTORY_CACHE_FENCES", "VICTORY_IMAGE_COHERENT"] {
            if let Some(value) = std::env::var_os(name) {
                bytes.extend_from_slice(name.as_bytes());
                bytes.push(b'=');
                bytes.extend_from_slice(value.as_encoded_bytes());
                bytes.push(0);
            }
        }
        // A host wave width taken from `VICTORY_HOST_WAVE` (only when set, so other runs keep their keys).
        if let Some(value) = std::env::var_os("VICTORY_HOST_WAVE") {
            bytes.extend_from_slice(b"VICTORY_HOST_WAVE=");
            bytes.extend_from_slice(value.as_encoded_bytes());
            bytes.push(0);
        }
        // And which clean-up passes ran over the translated code.
        bytes.extend_from_slice(&victory_shader::emit::shader_opt().to_le_bytes());
        // Renderer port 5 (`VICTORY_PORT_CBV=1`): constant buffers read as raw
        // bytes. Only when on, so the translations cached without it keep
        // their key.
        if victory_shader::emit::constants_raw() {
            bytes.extend_from_slice(b"VICTORY_PORT_CBV=1");
        }
        bytes
    });
    let mut hash = blake3::Hasher::new();
    hash.update(fixed);
    for value in victory_shader::emit::host_settings() {
        hash.update(&value.to_le_bytes());
    }
    *hash.finalize().as_bytes()
}

/// What tells one set of translations from another ([`build`]).
pub(crate) fn translation() -> [u8; 32] {
    build()
}

/// Puts the translation of `container` into the cache before any title asks
/// for it (`victory shaders`): whether it was there already, or why it does
/// not translate. The host is opened first, as a run does: what the card
/// can do decides the form of some programs.
pub fn precompile(container: &[u8]) -> Result<bool, String> {
    let _ = crate::host::host();
    let misses = MISSES.load(Ordering::Relaxed);
    let translate = |c: &[u8]| victory_shader::emit::translate(c).map_err(|e| e.to_string());
    match CACHE.get() {
        Some(cache) => cached(cache, &build(), container, translate, true)?,
        None => translate(container)?,
    };
    // (Counts other threads' misses too: the caller only adds them up.)
    Ok(MISSES.load(Ordering::Relaxed) == misses)
}

/// `VICTORY_CLAMP_PIXEL=<hash>[:<cap>|:drop<limit>][,...]`: pixel programs
/// (by the 16-digit hash logs give them) whose colour outputs are bounded
/// ([`victory_shader::emit::with_clamped_colour`]): `:<cap>` into
/// `[0, cap]` (1 when nothing is given), NaN to 0; `:drop<limit>` keeps
/// what is smaller than `limit` and makes a NaN, an infinity or anything
/// larger 0. They are translated afresh in every run and not cached, so
/// the cache's key stays the same.
fn clamp_for(container: &[u8]) -> Option<victory_shader::emit::ColourBound> {
    use victory_shader::emit::ColourBound;
    static LIST: OnceLock<Vec<(String, ColourBound)>> = OnceLock::new();
    let list = LIST.get_or_init(|| {
        std::env::var("VICTORY_CLAMP_PIXEL")
            .unwrap_or_default()
            .split(',')
            .filter_map(|item| {
                let (hash, how) = item.trim().split_once(':').unwrap_or((item.trim(), "1"));
                let how = how.trim();
                let (drop, number) = match how.strip_prefix("drop") {
                    Some(rest) => (true, rest),
                    None => (false, how),
                };
                let n: f32 = number.trim().parse().ok().filter(|c: &f32| c.is_finite() && *c > 0.0)?;
                let bound = if drop { ColourBound::Drop(n) } else { ColourBound::Cap(n) };
                (hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                    .then(|| (hash.to_ascii_lowercase(), bound))
            })
            .collect()
    });
    if list.is_empty() {
        return None;
    }
    let hash = blake3::hash(container).to_hex();
    list.iter().find(|(h, _)| h.as_str() == &hash[..16]).map(|&(_, bound)| bound)
}

/// `VICTORY_ZERO_INPUT=<hash>@s<slot>+<dword>[@...][,...]` (a diagnostic):
/// image reads of the named programs from those table places give 0
/// ([`victory_shader::emit::with_zeroed_inputs`]); translated afresh, not
/// cached.
fn zeroed_for(container: &[u8]) -> Option<Vec<(u8, u32)>> {
    static LIST: OnceLock<Vec<(String, Vec<(u8, u32)>)>> = OnceLock::new();
    let list = LIST.get_or_init(|| {
        std::env::var("VICTORY_ZERO_INPUT")
            .unwrap_or_default()
            .split(',')
            .filter_map(|item| {
                let mut parts = item.trim().split('@');
                let hash = parts.next()?.trim().to_ascii_lowercase();
                let places: Vec<(u8, u32)> = parts
                    .filter_map(|p| {
                        if let Some(at) = p.trim().strip_prefix("off") {
                            return Some((253, u32::from_str_radix(at, 16).ok()?));
                        }
                        if p.trim() == "heap" {
                            return Some((254, 0));
                        }
                        if let Some(dim) = p.trim().strip_prefix("dim") {
                            return Some((255, dim.parse().ok()?));
                        }
                        let (slot, dword) = p.trim().strip_prefix('s')?.split_once('+')?;
                        Some((slot.parse().ok()?, dword.parse().ok()?))
                    })
                    .collect();
                (hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && !places.is_empty())
                    .then_some((hash, places))
            })
            .collect()
    });
    if list.is_empty() {
        return None;
    }
    let hash = blake3::hash(container).to_hex();
    list.iter().find(|(h, _)| h.as_str() == &hash[..16]).map(|(_, p)| p.clone())
}

/// The translation of a console shader container, from the cache when
/// there is one.
pub(crate) fn host_shader(container: &[u8]) -> Result<HostShader, String> {
    // The per-program diagnostic switches below, looked for once: a run without them
    // skips them all.
    static ANY: OnceLock<bool> = OnceLock::new();
    let any = *ANY.get_or_init(|| {
        [
            "VICTORY_CLAMP_PIXEL",
            "VICTORY_ZERO_INPUT",
            "VICTORY_NAN_PROBE",
            "VICTORY_EXACT_FLOAT",
            "VICTORY_FTZ",
            "VICTORY_CLAMP_OPS",
            "VICTORY_DPP_OWN",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    });
    if !any {
        let translate = |c: &[u8]| victory_shader::emit::translate(c).map_err(|e| e.to_string());
        return match CACHE.get() {
            Some(cache) => cached(cache, &build(), container, translate, refresh_in_runs()),
            None => translate(container),
        };
    }
    let bound = clamp_for(container);
    let zeroed = zeroed_for(container);
    // `VICTORY_NAN_PROBE=<hash>@<from>-<to>` (byte offsets, hex): see
    // [`victory_shader::emit::with_nan_probe`].
    let probe = std::env::var("VICTORY_NAN_PROBE").ok().and_then(|v| {
        let (hash, range) = v.trim().split_once('@')?;
        let (from, to) = range.split_once('-')?;
        let from = usize::from_str_radix(from.trim_start_matches("0x"), 16).ok()?;
        let to = usize::from_str_radix(to.trim_start_matches("0x"), 16).ok()?;
        (blake3::hash(container).to_hex()[..16] == *hash.to_ascii_lowercase()).then_some((from, to))
    });
    // `VICTORY_EXACT_FLOAT=<hash>[:<bits>],...`: those programs translated with only these
    // bits of the host's float shortcuts (`victory_shader::emit::rdna`; 0 when not given).
    let exact = std::env::var("VICTORY_EXACT_FLOAT").ok().and_then(|v| {
        let own = blake3::hash(container).to_hex();
        v.split(',').find_map(|item| {
            let (hash, bits) = item.trim().split_once(':').unwrap_or((item.trim(), "0"));
            (hash.eq_ignore_ascii_case(&own[..16])).then(|| bits.parse::<u32>().unwrap_or(0))
        })
    });
    // `VICTORY_FTZ=<hash>,...`: those programs flush 32-bit denormals (`fp32-denorm-mode` ftz).
    let ftz = std::env::var("VICTORY_FTZ").is_ok_and(|v| {
        let own = blake3::hash(container).to_hex();
        v.split(',').any(|h| h.trim().eq_ignore_ascii_case(&own[..16]))
    });
    // `VICTORY_CLAMP_OPS=<hash>@<from>-<to>@<prefix>[,prefix]@<limit>` (hex offsets).
    let clamp_ops = std::env::var("VICTORY_CLAMP_OPS").ok().and_then(|v| {
        let parts: Vec<&str> = v.trim().split('@').collect();
        let [hash, range, prefixes, limit] = parts[..] else { return None };
        if !blake3::hash(container).to_hex()[..16].eq_ignore_ascii_case(hash) {
            return None;
        }
        let (from, to) = range.split_once('-')?;
        Some((
            usize::from_str_radix(from.trim_start_matches("0x"), 16).ok()?,
            usize::from_str_radix(to.trim_start_matches("0x"), 16).ok()?,
            prefixes.split(',').map(str::to_owned).collect::<Vec<_>>(),
            limit.parse::<f32>().ok()?,
        ))
    });
    // `VICTORY_DPP_OWN=<hash>,...`: those programs' DPP moves give each lane its own value.
    let dpp_own = std::env::var("VICTORY_DPP_OWN").is_ok_and(|v| {
        let own = blake3::hash(container).to_hex();
        v.split(',').any(|h| h.trim().eq_ignore_ascii_case(&own[..16]))
    });
    if bound.is_some() || zeroed.is_some() || probe.is_some() || exact.is_some() || ftz || clamp_ops.is_some() || dpp_own {
        let hash = &blake3::hash(container).to_hex()[..16];
        let places = zeroed.unwrap_or_default();
        let (done, met) = victory_shader::emit::with_zeroed_inputs(&places, || {
            let inner = || match bound {
                Some(bound) => victory_shader::emit::with_clamped_colour(bound, || victory_shader::emit::translate(container)),
                None => victory_shader::emit::translate(container),
            };
            let probed = || match probe {
                Some((from, to)) => match std::env::var("VICTORY_NAN_PROBE_ON").ok().filter(|p| !p.is_empty()) {
                    // (A name prefix such as `v_interp`: only those instructions are probed.)
                    Some(prefix) => {
                        let prefix: &'static str = Box::leak(prefix.into_boxed_str());
                        victory_shader::emit::with_nan_probe_on(prefix, || victory_shader::emit::with_nan_probe(from, to, inner))
                    }
                    None if std::env::var_os("VICTORY_NAN_PROBE_NAN").is_some() => {
                        victory_shader::emit::with_nan_only(|| victory_shader::emit::with_nan_probe(from, to, inner))
                    }
                    None => victory_shader::emit::with_nan_probe(from, to, inner),
                },
                None => inner(),
            };
            let limited = || match std::env::var("VICTORY_NAN_PROBE_LIMIT").ok().and_then(|l| l.parse::<f32>().ok()) {
                Some(limit) => victory_shader::emit::with_nan_probe_limit(limit, probed),
                None => probed(),
            };
            let signed = || {
                if std::env::var_os("VICTORY_NAN_PROBE_NEG").is_some() {
                    victory_shader::emit::with_nan_probe_neg(limited)
                } else {
                    limited()
                }
            };
            let clamped = || match clamp_ops.clone() {
                Some((from, to, prefixes, limit)) => victory_shader::emit::with_clamped_ops(from, to, prefixes, limit, signed),
                None => signed(),
            };
            let owned = || if dpp_own { victory_shader::emit::with_dpp_own(clamped) } else { clamped() };
            let flushed = || if ftz { victory_shader::emit::with_ftz(owned) } else { owned() };
            match exact {
                Some(bits) => victory_shader::emit::with_rdna_bits(bits, flushed),
                None => flushed(),
            }
        });
        tracing::info!(hash, ?bound, ?places, zeroed_reads = met, ?probe, ?exact, ftz, ?clamp_ops, dpp_own, "translated with a diagnostic change (VICTORY_CLAMP_PIXEL / VICTORY_ZERO_INPUT / VICTORY_NAN_PROBE)");
        return done.map_err(|e| e.to_string());
    }
    let translate = |c: &[u8]| victory_shader::emit::translate(c).map_err(|e| e.to_string());
    match CACHE.get() {
        Some(cache) => cached(cache, &build(), container, translate, refresh_in_runs()),
        None => translate(container),
    }
}

/// The cache entry's converter: the build and the container in, the
/// encoded translation out. `ran` tells a miss from a hit.
struct Translate<'a, F> {
    translate: F,
    /// The build of the translator, written into the entry.
    source: &'a str,
    ran: AtomicBool,
}

impl<F: Fn(&[u8]) -> Result<HostShader, String> + Send + Sync> Converter for Translate<'_, F> {
    fn id(&self) -> &'static str {
        "shader.dxil"
    }
    fn version(&self) -> u32 {
        // The build fingerprint in the input does the versioning.
        1
    }
    fn convert(&self, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.ran.store(true, Ordering::Relaxed);
        let shader = (self.translate)(&input[32..]).map_err(anyhow::Error::msg)?;
        Ok(encode(&shader, self.source))
    }
}

/// `refresh`: an entry an earlier build of the translator made is
/// translated again (and replaced) rather than served.
fn cached(
    cache: &Cache,
    build: &[u8; 32],
    container: &[u8],
    translate: impl Fn(&[u8]) -> Result<HostShader, String> + Send + Sync,
    refresh: bool,
) -> Result<HostShader, String> {
    cached_by(cache, build, SOURCE, container, translate, refresh)
}

/// [`cached`] with the build of the translator given (`source`).
fn cached_by(
    cache: &Cache,
    build: &[u8; 32],
    source: &str,
    container: &[u8],
    translate: impl Fn(&[u8]) -> Result<HostShader, String> + Send + Sync,
    refresh: bool,
) -> Result<HostShader, String> {
    let input = [&build[..], container].concat();
    let converter = Translate {
        translate,
        source,
        ran: AtomicBool::new(false),
    };
    let mut old: Option<HostShader> = None;
    for _ in 0..3 {
        let path = cache
            .get_or_convert(&converter, &input)
            .map_err(|e| format!("{e:#}"))?;
        if let Some((shader, made_by)) = std::fs::read(&path).ok().and_then(|b| decode(&b)) {
            let ran = converter.ran.load(Ordering::Relaxed);
            if !ran && made_by != source.as_bytes() {
                STALE.fetch_add(1, Ordering::Relaxed);
                if refresh && old.is_none() {
                    // Translated again in its place; what came out before
                    // is what the new one is compared with.
                    tracing::debug!(path = %path.display(), "shader cache entry of an earlier translator; translating again");
                    old = Some(shader);
                    let _ = std::fs::remove_file(&path);
                    continue;
                }
                static SAID: AtomicBool = AtomicBool::new(false);
                if !SAID.swap(true, Ordering::Relaxed) {
                    tracing::info!("shader translations of an earlier build of the translator (same version) are served; `victory shaders` translates them again");
                }
            }
            if let Some(old) = old.take() {
                REFRESHED.fetch_add(1, Ordering::Relaxed);
                if old != shader {
                    CHANGED.fetch_add(1, Ordering::Relaxed);
                }
            }
            let (counter, what) = if ran {
                (&MISSES, "translated and cached")
            } else {
                (&HITS, "from the cache")
            };
            counter.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(path = %path.display(), "shader {what}");
            return Ok(shader);
        }
        tracing::debug!(path = %path.display(), "unreadable shader cache entry; translating again");
        let _ = std::fs::remove_file(&path);
    }
    MISSES.fetch_add(1, Ordering::Relaxed);
    (converter.translate)(container)
}

const MAGIC: &[u8; 4] = b"VSH2";
const STAGES: [HwStage; 8] = [
    HwStage::Ls,
    HwStage::Hs,
    HwStage::Es,
    HwStage::Gs,
    HwStage::Vs,
    HwStage::Ps,
    HwStage::Ms,
    HwStage::Cs,
];

fn root_code(r: RootType) -> u32 {
    match r {
        RootType::Constants => 0,
        RootType::Descriptor => 1,
        RootType::Table => 2,
        RootType::ImmediateSampler => 3,
        RootType::ResourceHeap => 4,
        RootType::SamplerHeap => 5,
        RootType::Other(x) => 6 + x,
    }
}

fn root_type(code: u32) -> RootType {
    match code {
        0 => RootType::Constants,
        1 => RootType::Descriptor,
        2 => RootType::Table,
        3 => RootType::ImmediateSampler,
        4 => RootType::ResourceHeap,
        5 => RootType::SamplerHeap,
        x => RootType::Other(x - 6),
    }
}

/// Little-endian words and length-prefixed lists, after the build of the
/// translator that made the entry (`source`, length-prefixed).
fn encode(s: &HostShader, source: &str) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    let word = |out: &mut Vec<u8>, v: u32| out.extend(v.to_le_bytes());
    word(&mut out, source.len() as u32);
    out.extend(source.as_bytes());
    let stage = STAGES.iter().position(|&x| x == s.stage).unwrap_or(0) as u32;
    word(&mut out, stage);
    word(&mut out, s.dxil.len() as u32);
    out.extend(&s.dxil);
    word(&mut out, s.user_data.len() as u32);
    for u in &s.user_data {
        for v in [u.index, u32::from(u.slot), root_code(u.root), u.dword] {
            word(&mut out, v);
        }
    }
    word(&mut out, s.samplers.len() as u32);
    for sampler in &s.samplers {
        for &v in sampler {
            word(&mut out, v);
        }
    }
    word(&mut out, s.static_samplers.len() as u32);
    for t in &s.static_samplers {
        let [u, v, w] = t.address;
        let words = [
            t.filter,
            u,
            v,
            w,
            t.mip_lod_bias.to_bits(),
            t.max_anisotropy,
            t.comparison,
            t.border,
            t.min_lod.to_bits(),
            t.max_lod.to_bits(),
        ];
        for v in words {
            word(&mut out, v);
        }
    }
    for slots in [&s.root_cbvs, &s.root_srvs, &s.root_uavs] {
        word(&mut out, slots.len() as u32);
        out.extend(slots);
    }
    word(&mut out, s.root_indexed_views.len() as u32);
    for view in &s.root_indexed_views {
        for value in [
            u32::from(view.table_slot),
            u32::from(view.constant_slot),
            view.constant_dword,
            view.byte_offset,
            u32::from(view.shift),
        ] {
            word(&mut out, value);
        }
    }
    word(&mut out, s.half_views.len() as u32);
    for &(slot, dword) in &s.half_views {
        word(&mut out, u32::from(slot));
        word(&mut out, dword);
    }
    out
}

/// The shader and the build of the translator that made it.
fn decode(bytes: &[u8]) -> Option<(HostShader, Vec<u8>)> {
    struct Reader<'a>(&'a [u8]);
    impl<'a> Reader<'a> {
        fn take(&mut self, n: usize) -> Option<&'a [u8]> {
            let (head, rest) = self.0.split_at_checked(n)?;
            self.0 = rest;
            Some(head)
        }
        fn word(&mut self) -> Option<u32> {
            Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
        }
        fn list(&mut self) -> Option<Vec<u8>> {
            let n = self.word()? as usize;
            Some(self.take(n)?.to_vec())
        }
    }
    let mut r = Reader(bytes);
    if r.take(4)? != MAGIC {
        return None;
    }
    let made_by = r.list()?;
    let stage = *STAGES.get(r.word()? as usize)?;
    let dxil = r.list()?;
    let n = r.word()? as usize;
    let mut user_data = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        user_data.push(UserData {
            index: r.word()?,
            slot: u8::try_from(r.word()?).ok()?,
            root: root_type(r.word()?),
            dword: r.word()?,
        });
    }
    let n = r.word()? as usize;
    let mut samplers = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        samplers.push([r.word()?, r.word()?, r.word()?, r.word()?]);
    }
    let n = r.word()? as usize;
    let mut static_samplers = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        static_samplers.push(StaticSampler {
            filter: r.word()?,
            address: [r.word()?, r.word()?, r.word()?],
            mip_lod_bias: f32::from_bits(r.word()?),
            max_anisotropy: r.word()?,
            comparison: r.word()?,
            border: r.word()?,
            min_lod: f32::from_bits(r.word()?),
            max_lod: f32::from_bits(r.word()?),
        });
    }
    let root_cbvs = r.list()?;
    let root_srvs = r.list()?;
    let root_uavs = r.list()?;
    let n = r.word()? as usize;
    let mut root_indexed_views = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        root_indexed_views.push(RootIndexedView {
            table_slot: u8::try_from(r.word()?).ok()?,
            constant_slot: u8::try_from(r.word()?).ok()?,
            constant_dword: r.word()?,
            byte_offset: r.word()?,
            shift: u8::try_from(r.word()?).ok().filter(|&s| s < 32)?,
        });
    }
    let n = r.word()? as usize;
    let mut half_views = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        half_views.push((u8::try_from(r.word()?).ok()?, r.word()?));
    }
    r.0.is_empty().then_some((
        HostShader {
        stage,
        dxil,
        user_data,
        samplers,
        static_samplers,
        root_cbvs,
        root_srvs,
        root_uavs,
        root_indexed_views,
        half_views,
        },
        made_by,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shader() -> HostShader {
        HostShader {
            stage: HwStage::Ps,
            dxil: b"DXBC and the rest".to_vec(),
            user_data: vec![UserData {
                index: 8,
                slot: 1,
                root: RootType::Table,
                dword: 0,
            }],
            samplers: vec![[0x400, 0xc0_0000, 0x18f0_0000, 0]],
            static_samplers: vec![StaticSampler::from_words([
                0x400,
                0xc0_0000,
                0x18f0_0000,
                0,
            ])],
            root_cbvs: vec![2],
            root_srvs: vec![0, 3],
            root_uavs: vec![1],
            root_indexed_views: vec![RootIndexedView {
                table_slot: 1,
                constant_slot: 2,
                constant_dword: 3,
                byte_offset: 32,
                shift: 5,
            }],
            half_views: vec![(0, 8), (4, 0)],
        }
    }

    #[test]
    fn translations_come_back_from_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(dir.path()).unwrap();
        let build = [7u8; 32];
        let container = b"a console container".as_slice();
        let runs = AtomicU64::new(0);
        let translate = |_: &[u8]| {
            runs.fetch_add(1, Ordering::Relaxed);
            Ok(shader())
        };
        assert_eq!(cached(&cache, &build, container, translate, false), Ok(shader()));
        assert_eq!(cached(&cache, &build, container, translate, false), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 1, "the second came from disk");

        // Another build translates again.
        assert_eq!(cached(&cache, &[8; 32], container, translate, false), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 2);

        // A damaged entry is a miss, translated and stored again.
        for e in walk(dir.path()) {
            std::fs::write(e, b"VSH2 broken").unwrap();
        }
        assert_eq!(cached(&cache, &build, container, translate, false), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 3);
        assert_eq!(cached(&cache, &build, container, translate, false), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 3);

        // Failures are not stored.
        let fail = |_: &[u8]| Err::<HostShader, _>("unsupported".to_owned());
        assert!(cached(&cache, &build, b"other", fail, false).is_err());
    }

    fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if e.path().is_dir() {
                files.extend(walk(&e.path()));
            } else {
                files.push(e.path());
            }
        }
        files
    }

    #[test]
    fn entries_round_trip_and_reject_damage() {
        let bytes = encode(&shader(), "made by");
        assert_eq!(decode(&bytes), Some((shader(), b"made by".to_vec())));
        assert_eq!(decode(&bytes[..bytes.len() - 1]), None);
        assert_eq!(decode(&[bytes.as_slice(), &[0]].concat()), None);
        assert_eq!(decode(b"VSH0"), None);
        // Entries of the encoding before the translator's build was in them.
        let mut v1 = bytes.clone();
        v1[3] = b'1';
        assert_eq!(decode(&v1), None);
    }

    /// An entry an earlier build of the translator made, under the same
    /// version: served as it is; when refreshing translated again in its
    /// place, and a different result counted.
    #[test]
    fn entries_of_an_earlier_translator_are_served_or_refreshed() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(dir.path()).unwrap();
        let build = [9u8; 32];
        let container = b"a container of an earlier build".as_slice();
        let runs = AtomicU64::new(0);
        let old = || {
            let mut s = shader();
            s.dxil = b"the old translation".to_vec();
            s
        };
        let first = |_: &[u8]| {
            runs.fetch_add(1, Ordering::Relaxed);
            Ok(old())
        };
        assert_eq!(cached_by(&cache, &build, "build a", container, first, false), Ok(old()));
        let now = |_: &[u8]| {
            runs.fetch_add(1, Ordering::Relaxed);
            Ok(shader())
        };
        let (stale, refreshed, changed) = stale_counts();
        // The next build, same version: the old translation, not translated.
        assert_eq!(cached_by(&cache, &build, "build b", container, now, false), Ok(old()));
        assert_eq!(runs.load(Ordering::Relaxed), 1);
        // Refreshing: translated again and kept.
        assert_eq!(cached_by(&cache, &build, "build b", container, now, true), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 2);
        // Now the entry is this build's: served when refreshing too.
        assert_eq!(cached_by(&cache, &build, "build b", container, now, true), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 2);
        // (Other tests count too; these at least.)
        let (s, r, c) = stale_counts();
        assert!(s >= stale + 2 && r > refreshed && c > changed, "{s} {r} {c}");
    }
}
