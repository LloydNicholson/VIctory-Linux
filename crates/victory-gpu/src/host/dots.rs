//! Text in dots on the picture: white capitals of 5 by 7 dots on a black
//! box, painted on the CPU into an upload buffer when the text changes,
//! copied into a buffer in the card's memory, and from there onto the swap
//! chain's buffer at each present. No shader and no pipeline: a box can be
//! shown while the pipelines are what is being made. The diagnostics panel
//! (`panel`) and the loading bar of the pipelines made at the start
//! (`crate::pipeline::prepare`) are such boxes.

use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use super::{Host, HostList, transition};

/// A character's dots, and with the space beside and below it.
pub(super) const GLYPH: (u32, u32) = (5, 7);
pub(super) const CELL: (u32, u32) = (6, 9);
/// The box's rim in dots.
pub(super) const RIM: u32 = 3;

/// The texts are put into one of these in turn: the card reads the others,
/// written a while ago.
const UPLOADS: usize = 4;

/// Seven rows of five dots, the leftmost the highest bit. `=` is a whole
/// cell lit (a loading bar's filled part).
#[rustfmt::skip]
pub(super) const fn glyph(c: u8) -> [u8; 7] {
    match c {
        b'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        b'3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        b'4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        b'5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        b'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        b'7' => [0b11111, 0b00001, 0b00010, 0b00010, 0b00100, 0b00100, 0b00100],
        b'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        b'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        b'A' => [0b00100, 0b01010, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        b'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        b'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        b'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        b'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        b'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
        b'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        b'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        b'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        b'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        b'N' => [0b10001, 0b11001, 0b11001, 0b10101, 0b10011, 0b10011, 0b10001],
        b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        b'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        b'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        b'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        b'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        b'.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        b',' => [0, 0, 0, 0, 0b01100, 0b00100, 0b01000],
        b':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        b'/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        b'-' => [0, 0, 0, 0b11111, 0, 0, 0],
        b'+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        b'%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        b'(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        b')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        b'[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        b']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        b'!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        b'=' => [0b11111; 7],
        b' ' => [0; 7],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}

/// How many characters a line of a box holds, how many lines, and how many
/// rows of dots a bar under them takes (0: no bar).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) columns: u32,
    pub(super) lines: u32,
    pub(super) bar: u32,
}

impl Layout {
    /// The box's size in pixels, a dot being `scale` of them each way.
    pub(super) const fn size(self, scale: u32) -> (u32, u32) {
        (
            (self.columns * CELL.0 + 2 * RIM) * scale,
            (self.lines * CELL.1 + self.bar + 2 * RIM) * scale,
        )
    }

    /// Whether a dot of the picture of `lines` is white, the bar under them
    /// `fill` full: whole rows of dots as far as it is filled, its middle
    /// row past that.
    pub(super) fn lit(self, lines: &[String], fill: f64, x: u32, y: u32) -> bool {
        let (Some(x), Some(y)) = (x.checked_sub(RIM), y.checked_sub(RIM)) else {
            return false;
        };
        // The text's last column of dots is a gap; the bar ends where it does.
        let across = self.columns * CELL.0 - 1;
        if let Some(row) = y.checked_sub(self.lines * CELL.1).filter(|&r| r < self.bar) {
            let filled = (fill.clamp(0.0, 1.0) * f64::from(across)).round() as u32;
            return x < across && (x < filled || row == self.bar / 2);
        }
        if x % CELL.0 >= GLYPH.0 || y % CELL.1 >= GLYPH.1 || x / CELL.0 >= self.columns {
            return false;
        }
        let Some(&c) = lines.get((y / CELL.1) as usize).and_then(|l| l.as_bytes().get((x / CELL.0) as usize)) else {
            return false;
        };
        (glyph(c.to_ascii_uppercase())[(y % CELL.1) as usize] >> (GLYPH.0 - 1 - x % CELL.0)) & 1 != 0
    }

    /// The box as rows of `pitch` bytes.
    pub(super) fn painted(self, lines: &[String], fill: f64, scale: u32, colours: [&[u8]; 2], pitch: usize) -> Vec<u8> {
        let (width, height) = self.size(scale);
        let bytes = colours[0].len();
        let mut out = vec![0; pitch * height as usize];
        for y in 0..height / scale {
            let top = pitch * (y * scale) as usize;
            for x in 0..width / scale {
                let colour = colours[usize::from(self.lit(lines, fill, x, y))];
                for i in 0..scale {
                    let at = top + (x * scale + i) as usize * bytes;
                    out[at..at + bytes].copy_from_slice(colour);
                }
            }
            for i in 1..scale as usize {
                out.copy_within(top..top + pitch, top + i * pitch);
            }
        }
        out
    }
}

/// The bytes of a black and a white pixel of a swap chain's format.
pub(super) fn colours(format: DXGI_FORMAT) -> Option<[&'static [u8]; 2]> {
    Some(match format {
        DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_B8G8R8A8_UNORM => [&[0, 0, 0, 0xFF], &[0xFF; 4]],
        DXGI_FORMAT_R10G10B10A2_UNORM => [&[0, 0, 0, 0xC0], &[0xFF; 4]],
        // Halves of 0 and 1.
        DXGI_FORMAT_R16G16B16A16_FLOAT => [
            &[0, 0, 0, 0, 0, 0, 0, 0x3C],
            &[0, 0x3C, 0, 0x3C, 0, 0x3C, 0, 0x3C],
        ],
        _ => return None,
    })
}

/// Where a box goes on the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    /// The upper left corner (the diagnostics panel).
    #[cfg_attr(not(feature = "diagnostics"), allow(dead_code))]
    TopLeft,
    /// Across the middle, a twelfth of the picture above its bottom.
    LowMiddle,
}

/// The box's upper left corner on a picture of `width` by `height`.
pub(super) fn corner(place: Place, (w, h): (u32, u32), (width, height): (u32, u32)) -> (u32, u32) {
    match place {
        Place::TopLeft => (0, 0),
        Place::LowMiddle => (
            width.saturating_sub(w) / 2,
            height.saturating_sub(h).saturating_sub(height / 12),
        ),
    }
}

/// The buffers of a box that size and format.
struct Buffers {
    format: DXGI_FORMAT,
    scale: u32,
    pitch: u32,
    uploads: Vec<(ID3D12Resource, *mut u8)>,
    card: ID3D12Resource,
    /// The upload written last.
    turn: usize,
    /// It is not in the card's buffer yet.
    fresh: bool,
}

// SAFETY: free-threaded resources, and memory only written under the presenter's lock.
unsafe impl Send for Buffers {}

impl Buffers {
    fn new(host: &Host, layout: Layout, format: DXGI_FORMAT, scale: u32) -> Option<Buffers> {
        let (width, height) = layout.size(scale);
        let pitch = (width * colours(format)?[0].len() as u32).next_multiple_of(256);
        let bytes = u64::from(pitch) * u64::from(height);
        let uploads = (0..UPLOADS).map(|_| host.upload_buffer(bytes)).collect::<Option<Vec<_>>>()?;
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: bytes,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let mut card: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a buffer.
        unsafe {
            host.device
                .CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, D3D12_RESOURCE_STATE_COMMON, None, &mut card)
                .ok()?;
        }
        Some(Buffers { format, scale, pitch, uploads, card: card?, turn: 0, fresh: false })
    }

    fn write(&mut self, layout: Layout, lines: &[String], fill: f64) {
        let Some(colours) = colours(self.format) else { return };
        let picture = layout.painted(lines, fill, self.scale, colours, self.pitch as usize);
        self.turn = (self.turn + 1) % UPLOADS;
        // SAFETY: a mapped upload buffer of the picture's size, which the card read some presents ago.
        unsafe { std::ptr::copy_nonoverlapping(picture.as_ptr(), self.uploads[self.turn].1, picture.len()) };
        self.fresh = true;
    }
}

/// A box of text on the picture: its lines are set when they change, and
/// it is copied onto the swap chain's buffer at each present.
pub(super) struct Overlay {
    layout: Layout,
    place: Place,
    lines: Vec<String>,
    /// How full the bar is (a layout with one).
    fill: f64,
    changed: bool,
    buffers: Option<Buffers>,
    failed: bool,
}

impl Overlay {
    pub(super) fn new(layout: Layout, place: Place) -> Overlay {
        Overlay { layout, place, lines: Vec::new(), fill: 0.0, changed: false, buffers: None, failed: false }
    }

    /// The box shows `lines` from the next present on (none: no box).
    pub(super) fn set(&mut self, lines: Vec<String>) {
        if lines != self.lines {
            self.lines = lines;
            self.changed = true;
        }
    }

    /// The box shows `line` over its bar `fill` full (none: no box).
    pub(super) fn set_bar(&mut self, shown: Option<(String, f64)>) {
        let (lines, fill) = shown.map_or((Vec::new(), 0.0), |(line, fill)| (vec![line], fill));
        // The bar is painted again only when it moves a dot or more at most scales.
        if (fill - self.fill).abs() >= 0.002 || lines.is_empty() {
            self.fill = fill;
            self.changed = true;
        }
        self.set(lines);
    }

    /// Records the copy of the box onto `back`, a buffer of the swap chain of
    /// that size and format in the state to be copied into, a dot being
    /// `scale` pixels each way.
    pub(super) fn draw(
        &mut self,
        host: &Host,
        list: &HostList,
        back: &ID3D12Resource,
        (width, height): (u32, u32),
        format: DXGI_FORMAT,
        scale: u32,
    ) {
        if self.lines.is_empty() || self.failed {
            return;
        }
        if self.buffers.as_ref().is_none_or(|b| (b.format, b.scale) != (format, scale)) {
            // (What the card may still read stays: a picture's size changes once or twice in a run.)
            std::mem::forget(self.buffers.take());
            self.buffers = Buffers::new(host, self.layout, format, scale);
            self.failed = self.buffers.is_none();
            self.changed = true;
        }
        let Some(buffers) = &mut self.buffers else { return };
        if std::mem::take(&mut self.changed) {
            buffers.write(self.layout, &self.lines, self.fill);
        }
        let (w, h) = self.layout.size(scale);
        let (x, y) = corner(self.place, (w, h), (width, height));
        let list = &list.list;
        // SAFETY: an open list and live resources; the box is cut to the picture.
        unsafe {
            if std::mem::take(&mut buffers.fresh) {
                list.CopyResource(&buffers.card, &buffers.uploads[buffers.turn].0);
                list.ResourceBarrier(&[transition(
                    &buffers.card,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    D3D12_RESOURCE_STATE_COPY_SOURCE,
                )]);
            }
            let from = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&buffers.card),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: 0,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: format,
                            Width: w,
                            Height: h,
                            Depth: 1,
                            RowPitch: buffers.pitch,
                        },
                    },
                },
            };
            let to = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(back),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
            };
            let part = D3D12_BOX {
                left: 0,
                top: 0,
                front: 0,
                right: w.min(width - x),
                bottom: h.min(height - y),
                back: 1,
            };
            list.CopyTextureRegion(&to, x, y, 0, &from, Some(&part));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUT: Layout = Layout { columns: 40, lines: 9, bar: 0 };

    /// Every character the lines use has its own picture, and no two look
    /// alike.
    #[test]
    fn characters_differ() {
        let used: Vec<u8> = (b'0'..=b'9').chain(b'A'..=b'Z').chain(*b".,:/-+%()[]!= ").collect();
        for (i, a) in used.iter().enumerate() {
            assert_ne!(glyph(*a), glyph(b'#'), "{} has a picture", *a as char);
            assert!(glyph(*a).iter().all(|row| row >> GLYPH.0 == 0));
            for b in &used[i + 1..] {
                assert_ne!(glyph(*a), glyph(*b), "{} {}", *a as char, *b as char);
            }
        }
    }

    #[test]
    fn a_box_is_painted() {
        let lines = vec!["I".to_owned(), " i".to_owned()];
        for scale in [1, 2, 4] {
            let (width, height) = LAYOUT.size(scale);
            let pitch = (width * 4).next_multiple_of(256) as usize;
            let colours = colours(DXGI_FORMAT_R10G10B10A2_UNORM).unwrap();
            let picture = LAYOUT.painted(&lines, 0.0, scale, colours, pitch);
            assert_eq!(picture.len(), pitch * height as usize);
            let at = |x: u32, y: u32| &picture[y as usize * pitch + x as usize * 4..][..4];
            // The rim is black, the first letter's top row lit in its middle.
            assert_eq!(at(0, 0), colours[0]);
            assert_eq!(at((RIM + 2) * scale, RIM * scale), colours[1]);
            assert_eq!(at((RIM + 2) * scale + scale - 1, RIM * scale + scale - 1), colours[1]);
            assert_eq!(at(RIM * scale, RIM * scale), colours[0]);
            // Small letters are shown as capitals, a line and a cell on.
            assert_eq!(at((RIM + CELL.0 + 2) * scale, (RIM + CELL.1) * scale), colours[1]);
            // Nothing past a line's end.
            assert_eq!(at((RIM + CELL.0 * 2 + 2) * scale, (RIM + CELL.1) * scale), colours[0]);
        }
    }

    /// A bar under the line: whole rows as far as it is filled, its middle
    /// row past that, nothing past the text's width.
    #[test]
    fn a_bar_is_painted_under_the_line() {
        let layout = Layout { columns: 10, lines: 1, bar: 3 };
        let lines = ["X".to_owned()];
        let (top, across) = (RIM + CELL.1, 10 * CELL.0 - 1);
        let lit = |fill, x, y| layout.lit(&lines, fill, RIM + x, top + y);
        assert_eq!(layout.size(1), (10 * CELL.0 + 2 * RIM, CELL.1 + 3 + 2 * RIM));
        // Half full: all three rows to the middle, then the middle row only.
        let half = across / 2;
        assert!((0..3).all(|y| lit(0.5, 0, y) && lit(0.5, half, y)));
        assert!(!lit(0.5, half + 1, 0) && lit(0.5, half + 1, 1) && !lit(0.5, half + 1, 2));
        assert!(lit(0.0, across - 1, 1) && !lit(0.0, across, 1));
        assert!((0..3).all(|y| lit(1.0, across - 1, y)));
        // The text above it is as before.
        assert!(layout.lit(&lines, 0.5, RIM, RIM) && !layout.lit(&lines, 0.5, RIM + 2, RIM));
    }

    /// The low box sits across the middle, above the bottom, and inside a
    /// picture smaller than itself.
    #[test]
    fn the_low_box_is_centred_above_the_bottom() {
        assert_eq!(corner(Place::TopLeft, (100, 50), (1920, 1080)), (0, 0));
        assert_eq!(corner(Place::LowMiddle, (100, 50), (1920, 1080)), (910, 1080 - 50 - 90));
        assert_eq!(corner(Place::LowMiddle, (3000, 2000), (1920, 1080)), (0, 0));
    }
}
