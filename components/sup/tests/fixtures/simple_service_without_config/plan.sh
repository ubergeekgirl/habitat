pkg_name=simple_service_without_config
pkg_origin=test
pkg_version=0.0.1
pkg_license=('Apache2')
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_source=nosuchfile.tar.gz
pkg_deps=(chef/gpgme chef/libassuan chef/libgpg-error)
pkg_bin_dirs=(bin)
pkg_service_run="bin/simple_service_without_config"
pkg_gpg_key=3853DA6B

do_begin() {
  archive="$BLDR_SRC_CACHE/${pkg_name}-${pkg_version}.tar.bz2"
  tar -cjvf $archive --exclude 'plans' --exclude '.git' --exclude '.gitignore' --exclude 'target' --transform "s,^\.,simple_service_without_config-0.0.1," .
  pkg_shasum=$(trim $(sha256sum $archive | cut -d " " -f 1))
}

do_download() {
  return 0
}

do_build() {
  return 0
}

do_install() {
  cp -r /src/components/sup/target/debug/hab-sup $pkg_prefix/bin
  cp -r $BLDR_SRC_CACHE/$pkg_dirname/bin $pkg_prefix
  chmod 755 $pkg_path/bin
  chmod 755 $pkg_path/bin/*
}
