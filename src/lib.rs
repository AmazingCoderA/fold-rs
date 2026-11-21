//! Ядро программы `Fold` (Viktor Romanov, 2007), переписанное на Rust.
//!
//! Оригинал: 32-битный Delphi/GUI, упакованный ASPack. Все константы и
//! геометрия ниже восстановлены из дампа распакованного процесса:
//!
//!   карта       : 8000 x 4000 м   (0x46AE48 = 8000, 0x46AE4C = 4000)
//!   шаг выборки : 200 м           (0x46742C = 200.0)
//!   V ср по умолчанию: 1500 м/с   (Edit2 в DFM)
//!   качество по умолчанию: 95     (Edit3 в DFM)
//!   эллиптические веса: 0.4 / 0.6 (xword 0x4663E8 / 0x4663F4)
//!   порог 210, масштаб 26, база 250, сдвиг 2000 (0x466400..0x466418)

pub mod grid;
#[cfg(feature = "gui")]
pub mod gui;
pub mod io;
pub mod model;
pub mod velocity;

pub use grid::{build_map, MapGrid};
pub use model::{Pick, Profile, Settings};
