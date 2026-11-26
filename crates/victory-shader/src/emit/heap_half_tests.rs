//! A 16-bit load (`buffer_load_format_d16_x`) through a typed buffer view the
//! program picks from the resource heap: the face network reads its halves
//! from `R16_UINT` views this way too. Its integer view is known from the
//! view-stride table (`VIEW_INTEGER`, VIctory 4243a96a), and the translation
//! keeps a float-typed handle there and relies on the driver passing the
//! 16 bits of the integer view through it (checked by upstream on an
//! RX 6700 XT and WARP, under Windows). This runs it on the host's own card:
//! under Proton that is vkd3d-proton on RADV.

use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_FORMAT_R16_FLOAT, DXGI_FORMAT_R16_UINT};

use super::host::{Dispatch, Host, Root, Stride, View};
use super::{HostShader, VIEW_INTEGER, lower, validator};
use crate::native::{
    ComputeIo, HwStage, NativeProgram, ResourceType, RootParam, RootType, StageIo, UsedResource,
};
use crate::{io, ir, resource, structure};

/// User data: s0 the heap index of a typed buffer (a root constant), s1 the
/// resource heap, s2 a table with a raw UAV. Lane `l` loads element `l`
/// with `buffer_load_format_d16_x` and stores the dword to dword `l`.
const HEAP_D16_X_WAVE32: &[u32] = &[
    0x8F04_8500, // s_lshl_b32 s4, s0, 5
    0xBE86_0301, // s_mov_b32 s6, s1
    0xBE87_0380, // s_mov_b32 s7, 0
    0xF408_0203,
    0x0800_0000, // s_load_dwordx4 s[8:11], s[6:7], s4
    0xBE8C_0302, // s_mov_b32 s12, s2
    0xBE8D_0380, // s_mov_b32 s13, 0
    0xF408_0406,
    0xFA00_0000, // s_load_dwordx4 s[16:19], s[12:13], 0x0
    0x3402_0082, // v_lshlrev_b32 v1, 2, v0
    0x7E04_0280, // v_mov_b32 v2, 0
    0xBF8C_C07F, // s_waitcnt lgkmcnt(0)
    0xE200_2000,
    0x8002_0200, // buffer_load_format_d16_x v2, v0, s[8:11], 0 idxen
    0xBF8C_3F70, // s_waitcnt vmcnt(0)
    0xE070_1000,
    0x8004_0201, // buffer_store_dword v2, v1, s[16:19], 0 offen
    0xBF81_0000, // s_endpgm
];

fn param(root: RootType, user_data: u8) -> RootParam {
    RootParam { root, table_offset_dwords: 0, sgprs: 1, base_register: 0, user_data, slot: user_data }
}

fn build() -> HostShader {
    let native = NativeProgram {
        stage: HwStage::Cs,
        wave64: false,
        vgprs: 8,
        layout: vec![
            param(RootType::Constants, 0),
            param(RootType::ResourceHeap, 1),
            param(RootType::Table, 2),
        ],
        used: vec![UsedResource {
            kind: ResourceType::Uav,
            root: RootType::Table,
            table_offset_dwords: 0,
            lower_bound: 0,
            stride: 0,
            dwords: 4,
            user_data: 2,
            slot: 2,
        }],
        io: StageIo::Cs(ComputeIo { rsrc2: 3 << 1, threads: [32, 1, 1] }),
        code: HEAP_D16_X_WAVE32.iter().flat_map(|w| w.to_le_bytes()).collect(),
    };
    let p = ir::build(&native.code, native.wave64, native.vgprs).expect("ir::build");
    let st = structure::structure(&p).expect("structure");
    let res = resource::track(&p, &native.layout, native.stage, Some(&st)).expect("track");
    let io = io::raise(&p, &native).expect("io::raise");
    let lowered = lower::lower(&lower::Parts {
        native: &native,
        p: &p,
        st: &st,
        io: &io,
        res: &res,
        inputs: Vec::new(),
        outputs: Vec::new(),
        prims: Vec::new(),
        mesh_stride: 0,
    })
    .expect("lower");
    assert!(lowered.half_views.is_empty(), "a view picked from the heap names no table place");
    let shader = HostShader {
        stage: HwStage::Cs,
        dxil: lowered.dxil,
        user_data: lowered.user_data,
        samplers: lowered.samplers,
        static_samplers: Vec::new(),
        root_cbvs: lowered.root_cbvs,
        root_srvs: lowered.root_srvs,
        root_uavs: lowered.root_uavs,
        root_indexed_views: res.root_indexed_views,
        half_views: lowered.half_views,
    };
    if let Ok(mut v) = validator::Validator::load(validator::Validator::default_path().as_deref()) {
        v.validate(&mut shader.dxil.clone()).expect("DXIL validation failed");
    }
    shader
}

/// An `R16_UINT` view's 16 bits as they are, an `R16_FLOAT` view's half as
/// it is, each read through a view picked from the heap, on the card and
/// on WARP (each said apart, so a driver that converts shows by name).
#[test]
fn d16_x_loads_through_heap_picked_views_keep_their_bits() {
    let shader = build();
    let pack = |h: &dyn Fn(u32) -> u32| -> Vec<u32> { (0..16u32).map(|k| h(2 * k) | h(2 * k + 1) << 16).collect() };
    // Patterns a half would change: signalling NaN, denormals, all ones.
    let uint = |l: u32| [0xF000 | l, 0x7C01 + l, 0x0001 + l, 0xFFFF - l][l as usize % 4];
    let half = |l: u32| 0x3C00 + 5 * l;
    let cases: [(DXGI_FORMAT, u32, Vec<u32>, Vec<u32>); 2] = [
        (DXGI_FORMAT_R16_UINT, 2 | VIEW_INTEGER, pack(&uint), (0..32).map(uint).collect()),
        (DXGI_FORMAT_R16_FLOAT, 2, pack(&half), (0..32).map(half).collect()),
    ];
    let _one = super::valu_tests::ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut failed = Vec::new();
    let mut ran = 0;
    for (label, warp) in [("hardware", false), ("warp", true)] {
        let Ok(host) = Host::new(warp) else {
            eprintln!("no SM 6.6 {label} device; skipped");
            continue;
        };
        let (root, pso) = host
            .compute_pipeline(&shader)
            .unwrap_or_else(|e| panic!("{label}: compute_pipeline failed: {e}"));
        for (format, stride, data, want) in &cases {
            let mut ud = vec![0u32; shader.user_data_dwords().max(1) as usize];
            for u in &shader.user_data {
                ud[u.index as usize] = match u.root {
                    // The typed buffer is at heap index 1, the UAV's table at 0.
                    RootType::Constants => 1,
                    RootType::Table => 0,
                    other => panic!("unexpected user data {other:?}"),
                };
            }
            let views = [View::RawUav(32), View::Buffer { data, stride: Stride::Format(*format, 2) }];
            let words = host
                .dispatch(
                    &root,
                    &pso,
                    &Dispatch {
                        roots: vec![(0, Root::Constants(ud)), (1, Root::View { srv: true, data: vec![4, *stride] })],
                        views: &views,
                        samplers: &[],
                        groups: [1, 1, 1],
                    },
                )
                .unwrap_or_else(|e| panic!("{label}: dispatch failed: {e}"));
            let got: Vec<u32> = words[0].iter().map(|w| w & 0xFFFF).collect();
            eprintln!("{label} {format:?}: first lanes {:04x?} (want {:04x?})", &got[..8], &want[..8]);
            if got != *want {
                failed.push(format!("{label} {format:?}"));
            }
        }
        ran += 1;
    }
    assert!(ran > 0, "neither hardware nor WARP SM 6.6 device available");
    assert!(failed.is_empty(), "16 bits not kept through a heap-picked view: {failed:?}");
}
