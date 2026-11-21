# Fold (Rust port)

Rust-реализация ядра программы **Fold — «Построение структурной карты отражающего горизонта»** (Viktor Romanov, 2007).

## Что было сделано

1. Оригинал `03_Fold.exe` — 32-битный Delphi/VCL GUI, упакован ASPack. Две копии в архиве побайтово идентичны (SHA256 `5E607CA9...71A96`).
2. Снят дамп распакованного процесса (`analysis/memory/unpacked_image.bin`) и собран PE для анализа ресурсов.
3. Из DFM восстановлены форма, Element-ы и обработчики: `begin_prf`, `paint_prf`, `end_prf`, `coora`, `N1Click`, `C1Click`, `Delete_ProfileClick`, `UnDelete_ProfileClick`, `EntryHandleClick`.
4. По графу вызовов (66 функций) извлечены FPU-операции и константы:
   - карта 8000×4000 м (`0x46AE48`, `0x46AE4C`);
   - шаг 200 м (`0x46742C`), делитель 2 (`0x467428`), π (`0x467418`), 180 (`0x467424`);
   - веса 0.4 / 0.6 (`0x4663E8`, `0x4663F4`), радиус 2000 м (`0x466418`);
   - V ср 1500 м/с и качество 95 — значения по умолчанию из DFM.
5. Написано Rust-ядро с CLI и PGM-выводом.

## Репозиторий

https://github.com/AmazingCoderA/fold-rs

## Сборка и запуск

```powershell
cargo build --release
.\target\release\fold.exe examples\demo.txt map.pgm
.\target\release\fold.exe --bench
.\target\release\fold-gui.exe        # GUI-клон формы
cargo test --release
```

На Linux: `cargo build --release`, затем `./target/release/fold-gui`.
Для GUI нужны `libx11-dev libxi-dev libgl1-mesa-dev libudev-dev libxkbcommon-dev libwayland-dev pkg-config`.

### AUR (Arch Linux)

```bash
yay -S fold-rs-git
# или вручную:
cd packaging/aur && makepkg -si
```

Формат входа (`#` — комментарий):

```
v 1500        # средняя скорость, м/с
q 95          # качество, %
profile x0 y0 x1 y1
pick трасса время
...
end
```

## Производительность

| | Время |
|---|---|
| Старт оригинального `03_Fold.exe` (до отображения окна) | ~370 мс |
| Rust, 15 профилей × 200 пиков, одно построение карты | **2.1 мс** |
| Rust, демо (3 профиля) | 0.36 мс |

Ограничение в 15 профилей взято из UI оригинального приложения.

## Структура

- `src/model.rs` —剖面/пики/настройки;
- `src/velocity.rs` — пересчёт `t → H` (`H = V·t/2`);
- `src/grid.rs` — интерполяция и взвешенная карта глубин;
- `src/io.rs` — парсер входа и запись PGM (16-bit grayscale);
- `analysis/` — скрипты дампа, DFM-парсер, дизассемблер и отчёты.

## Оговорка

Это независимая реконструкция по бинарнику: точная формула нелинейной фильтрации оригинала (`0x466250`, `0x466980`) восстановлена приближённо по константам и структуре FPU-кода. Геометрия, единицы и интерфейс сохранены; при наличии эталонных данных корректируется весовая функция в `src/grid.rs`.
