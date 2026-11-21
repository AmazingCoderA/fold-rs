# Fold — Rust-порт программы 2007 года

[![Build](https://github.com/AmazingCoderA/fold-rs/actions/workflows/build.yml/badge.svg)](https://github.com/AmazingCoderA/fold-rs/actions/workflows/build.yml)

Полный независимый порт программы **Fold — «Построение структурной карты отражающего горизонта»** (Viktor Romanov, 2007), написанной на Delphi/VCL и упакованной ASPack.

**Цель:** один в один повточнить интерфейс и логику, но считать быстрее оригинала и работать там, где его нет: Windows, Linux, macOS, Android.

---

## Что уже есть

| Целевая plataforma | Статус | Артефакт |
|---|---|---|
| Windows x64 | ✅ CI зелёный | `fold-windows-x64.tar.gz` |
| Linux x64 | ✅ CI зелёный | `fold-linux-x64.tar.gz` |
| macOS arm64 | ✅ CI зелёный | `fold-macos-arm64.tar.gz` |
| Android (arm64-v8a, armeabi-v7a, x86_64) | ✅ JNI + Kotlin UI | `fold-android-apk` в CI |
| Arch Linux AUR | ✅ PKGBUILD готов | `fold-rs-git` |

**Релизы:** https://github.com/AmazingCoderA/fold-rs/releases

---

## GUI 1:1

Интерфейс восстановлен по предоставленному скриншоту (894×656):

```
┌─────────────────────────────────────────────────────────────┐
│ Структура  Профили  Стереть таблицу                          │
├────────────────┬────────────────────────────────────────────┤
│ Параметры      │  Построение структурной карты ...           │
│ структуры      │  Структурная карта              X, м        │
│ V ср 1200      │ ┌────────────────────────────────────────┐ │
│ f 10           │ │        цветная карта + сетка           │ │
│                │ │  профили рисуются мышью               │ │
│ Координаты     │ └────────────────────────────────────────┘ │
│ профиля        │  Временной разрез                 трассы   │
│ Профиль 3      │ ┌────────────────────────────────────────┐ │
│ X 3100 Y 100   │ │  пики ставятся кликом, ось 1..10      │ │
│ [<] [>]        │ └────────────────────────────────────────┘ │
│ Таблица        │                        Viktor Romanov, 2007│
│ корреляции     │                                            │
│ t0 = 3.465 с   │                                            │
│ Качество       │                                            │
└────────────────┴────────────────────────────────────────────┘
```

- цветовая палитра карты повторяет оригинальную (cyan → green → yellow → orange);
- V ср по умолчанию 1200 м/с (как на скриншоте), качество 95, профиль 3;
- кнопки **Назад / Далее / Стереть таблицу / Удалить все** работают;
- пересчёт происходит только при изменении данных — никаких лагов в idle.

---

## Сборка и запуск

### Desktop

```powershell
cargo build --release
.\target\release\fold.exe examples\demo.txt map.pgm   # CLI
.\target\release\fold-gui.exe                         # GUI
.\target\release\fold.exe --bench                     # бенчмарк
cargo test --release
```

На Linux для GUI нужны:
```bash
sudo apt install pkg-config libx11-dev libxi-dev libgl1-mesa-dev libudev-dev libxkbcommon-dev libwayland-dev
cargo build --release
```

### Android

Нативный код считает карту, Kotlin-интерфейс рисует её и обрабатывает касания:

```bash
cargo install cargo-ndk
cargo ndk --manifest-path android-rs/Cargo.toml \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o android/app/src/main/jniLibs build --release
gradle -p android assembleDebug
```

APK появится в `android/app/build/outputs/apk/debug/`. В CI это делает job `android`.

### AUR (Arch Linux)

```bash
yay -S fold-rs-git
# или локально:
cd packaging/aur && makepkg -si
```

Инструкция по публикации пакета — `packaging/aur/README.md`.

---

## Формат данных

```
v 1500        # средняя скорость, м/с
q 95          # качество, %
profile x0 y0 x1 y1
pick трасса время
...
end
```

`#` — комментарий. Пример: `examples/demo.txt`.

---

## Производительность

| Что меряли | Время |
|---|---|
| Старт оригинального `03_Fold.exe` | ~370 мс |
| Rust, 15 профилей × 200 пиков | **2.1 мс** |
| Rust, демо | ~0.4 мс |

Алгоритм: `H = V·t/2`, шаг 200 м, карта 8000×4000 м, веса 0.4/0.6, радиус 2000 м — константы вытащены из распакованного бинарника.

---

## Структура

- `src/model.rs`, `src/velocity.rs`, `src/grid.rs` — ядро;
- `src/io.rs` — парсер и PGM-вывод;
- `src/gui.rs` — egui-клон формы;
- `android-rs/` — JNI-мост на Rust;
- `android/` — Gradle/Kotlin приложение;
- `.github/workflows/` — CI и релизы;
- `packaging/aur/` — PKGBUILD;
- `analysis/` — скрипты дампа, DFM-парсер, дизассемблер и отчёты.

## Оговорка

Нелинейный фильтр оригинала (`0x466250` / `0x466980`) восстановлен приближённо по константам и FPU-коду. Геометрия, единицы и интерфейс сохранены; при эталонных картах корректируется весовая функция в `src/grid.rs`.
