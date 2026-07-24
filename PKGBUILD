# Maintainer: hello@zurat.dev
pkgname=dotted
pkgver=1.0.0
pkgrel=1
pkgdesc="A simple, templateless, multi-[device|repo|user|distro] dotfile manager that is highly shareable and tracks system packages."
arch=('x86_64')
url="https://github.com/z00rat/dotted"
license=('AGPL-3.0-only')

provides=('dotted')
conflicts=('dotted')

build() {
  true
}

package() {
  install -Dm755 "$srcdir/target/release/$pkgname" "$pkgdir/usr/bin/$pkgname" 2>/dev/null || \
  install -Dm755 "$startdir/target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  install -Dm644 "$startdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 "$startdir/target/completions/dotted.bash" "$pkgdir/usr/share/bash-completion/completions/dotted"
  install -Dm644 "$startdir/target/completions/_dotted" "$pkgdir/usr/share/zsh/site-functions/_dotted"
  install -Dm644 "$startdir/target/completions/dotted.fish" "$pkgdir/usr/share/fish/vendor_completions.d/dotted.fish"
}
