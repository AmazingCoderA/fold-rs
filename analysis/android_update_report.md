# Android: нормальное обновление и совпадение с desktop

## Почему APK не обновлялся

CI собирал `assembleDebug`. Debug-подпись Gradle создаётся в `~/.android/debug.keystore`
на каждом runner'е, поэтому каждый релиз имел другой сертификат. Android разрешает
обновление только при совпадении подписи и `applicationId`, поэтому установка
очередного APK падала с `INSTALL_FAILED_UPDATE_INCOMPATIBLE` / «App not installed».

## Что исправлено

* `android/app/fold-release.jks` — постоянный key store (alias `fold`, RSA-2048),
  используется и для `debug`, и для `release` сборок.
* `android/app/build.gradle` берёт `versionName`/`versionCode` из env:
  `v0.10.7` → `0.10.7` / `10007` (`major*100000 + minor*1000 + patch`).
* Workflow Android job считает эти значения из `GITHUB_REF_NAME`, собирает
  `assembleRelease` и публикует `app-release.apk` как `fold-android-universal.apk`.
* Кнопка **Обновить** открывает `.../releases/latest`; обычный тап по APK теперь
  обновляет приложение без удаления.

Уже установленные старые debug-сборки подписаны другими ключами — их нужно удалить
один раз. После этого все будущие APK обновляются штатно.

## Визуальное совпадение с desktop

Раньше Android рисовал упрощённую карту из публичного `build_map` (41×21) и
«разрез» из трёх фиктивных пиков. Теперь JNI отдаёт готовые ARGB-буферы:

* `buildMapPixels(velocity, quality, angle)` → `[800, 400, ARGB...]`, та же
  растеризация `0x46641C` и палитра `map_color_classic`, что и в GUI;
* `buildSectionPixels(...)` → `[800, 201, t_min_bits, t_max_bits, n, ARGB...]`,
  та же растеризация `0x465D68` (300 отсчётов, RMS по первой трассе);
* Kotlin только масштабирует bitmap с `isFilterBitmap = false`, рисует сетку
  8×10, профили, подписи и маркеры пиков `HACHA` 12×4.

Ось Y карты отражена (`mapPoint`), ось времени разреза растёт вниз, как в
original. Пики и клик по разрезу используют те же 11 условных трасс, что и
desktop GUI (`0x468780`), поэтому маркеры совпадают с PC. `V` ограничен
`1000..7000`, качество `0.1..9.99`, координаты X/Y — `0..8000` / `0..4000`.

Надёжность сборки: CI явно фиксирует Gradle 8.9 (совместим с AGP 8.6.1),
ключа подписи лежит в репозитории, `publish` работает только по тегу `v*`,
а `versionCode` из тега всегда выше debug-сборок. Движение пальцем по карте
больше не перехватывается `ScrollView`, `ACTION_CANCEL` не оставляет «висячую»
линию профиля.
