# Публикация в AUR

Пакет `fold-rs-git` уже собран в `packaging/aur/PKGBUILD`.

## Первый раз

1. Создай аккаунт на https://aur.archlinux.org и добавь свой SSH-ключ.
2. Установи `git` и `makepkg` (`pacman -S base-devel git`).
3. Клонируй категорию пакетов:
   ```bash
   git clone ssh://aur@aur.archlinux.org/fold-rs-git.git
   cd fold-rs-git
   ```
4. Скопируй `PKGBUILD` из этого каталога и **обязательно перегенерируй**
   `.SRCINFO` на свежем клоне (у `-git` пакета `pkgver()` плавает):
   ```bash
   cp /path/to/RustFold/packaging/aur/PKGBUILD .
   makepkg --printsrcinfo > .SRCINFO
   ```

   Лежащий здесь `.SRCINFO` — только справочный снимок на момент релиза;
   перед публикацией его надо пересоздать, иначе AUR покажет старую версию.
5. Проверь сборку:
   ```bash
   makepkg -si
   ```
6. Опубликуй:
   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "Initial import"
   git push
   ```

## Обновление

При выходе новой версии обнови `pkgver`/`pkgrel` (для `-git` пакета `pkgver()` считается из git автоматически), перегенерируй `.SRCINFO` командой `makepkg --printsrcinfo > .SRCINFO` и запушь.

## Установка пользователями

```bash
yay -S fold-rs-git
# или
paru -S fold-rs-git
```
