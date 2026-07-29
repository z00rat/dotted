# Maintainer: hello@zurat.dev
#
# Local packaging recipe for GitHub Release artifacts.
# `just package-arch` copies this recipe and all inputs into an isolated
# target/arch-package/ directory before running makepkg.
# Not an AUR PKGBUILD (no remote source tarball).

pkgname=dotted
pkgver=1.0.4
pkgrel=1
pkgdesc="A simple, templateless, multi-[device|repo|user|distro] dotfile manager that is highly shareable and tracks system packages & services."
arch=('x86_64')
url="https://github.com/z00rat/dotted"
license=('AGPL-3.0-only')
depends=('glibc' 'gcc-libs')
options=('strip')

# Staged beside the copied PKGBUILD by justfile.
source=(
	"dotted"
	"LICENSE"
	"dotted.bash"
	"_dotted"
	"dotted.fish"
)
# Local-only staging: integrity is guaranteed by the release build that wrote these files.
sha256sums=('SKIP' 'SKIP' 'SKIP' 'SKIP' 'SKIP')

package() {
	install -Dm755 "$srcdir/dotted" "$pkgdir/usr/bin/dotted"
	install -Dm644 "$srcdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
	install -Dm644 "$srcdir/dotted.bash" "$pkgdir/usr/share/bash-completion/completions/dotted"
	install -Dm644 "$srcdir/_dotted" "$pkgdir/usr/share/zsh/site-functions/_dotted"
	install -Dm644 "$srcdir/dotted.fish" "$pkgdir/usr/share/fish/vendor_completions.d/dotted.fish"
}
