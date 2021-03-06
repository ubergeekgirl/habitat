// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate hab;
extern crate habitat_core as hcore;
extern crate habitat_common as common;
extern crate handlebars;
#[macro_use]
extern crate log;

use std::env;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;
use std::thread;

use clap::{ArgMatches, Shell};

use common::ui::{Coloring, UI, NOCOLORING_ENVVAR, NONINTERACTIVE_ENVVAR};
use hcore::env as henv;
use hcore::crypto::{init, default_cache_key_path, SigKeyPair};
use hcore::crypto::keys::PairType;
use hcore::fs::{cache_artifact_path, cache_analytics_path, cache_key_path, FS_ROOT_PATH};
use hcore::service::ServiceGroup;
use hcore::package::PackageIdent;
use hcore::url::{DEFAULT_DEPOT_URL, DEPOT_URL_ENVVAR};

use hab::{analytics, cli, command, config, AUTH_TOKEN_ENVVAR, ORIGIN_ENVVAR, PRODUCT, VERSION};
use hab::error::{Error, Result};

/// Makes the --org CLI param optional when this env var is set
const HABITAT_ORG_ENVVAR: &'static str = "HAB_ORG";

const FS_ROOT_ENVVAR: &'static str = "FS_ROOT";

const DEFAULT_BINLINK_DIR: &'static str = "/bin";

fn main() {
    env_logger::init().unwrap();
    let mut ui = ui();
    thread::spawn(|| analytics::instrument_subcommand());
    if let Err(e) = start(&mut ui) {
        ui.fatal(e).unwrap();
        std::process::exit(1)
    }
}

fn start(ui: &mut UI) -> Result<()> {
    try!(exec_subcommand_if_called(ui));

    let (args, remaining_args) = raw_parse_args();
    debug!("clap cli args: {:?}", &args);
    debug!("remaining cli args: {:?}", &remaining_args);
    let app_matches =
        cli::get().get_matches_from_safe_borrow(&mut args.iter()).unwrap_or_else(|e| {
            analytics::instrument_clap_error(&e);
            e.exit();
        });
    match app_matches.subcommand() {
        ("cli", Some(matches)) => {
            match matches.subcommand() {
                ("setup", Some(_)) => try!(sub_cli_setup(ui)),
                ("completers", Some(m)) => try!(sub_cli_completers(m)),
                _ => unreachable!(),
            }
        }
        ("install", Some(m)) => try!(sub_pkg_install(ui, m)),
        ("origin", Some(matches)) => {
            match matches.subcommand() {
                ("key", Some(m)) => {
                    match m.subcommand() {
                        ("download", Some(sc)) => try!(sub_origin_key_download(ui, sc)),
                        ("export", Some(sc)) => try!(sub_origin_key_export(sc)),
                        ("generate", Some(sc)) => try!(sub_origin_key_generate(ui, sc)),
                        ("import", Some(_)) => try!(sub_origin_key_import(ui)),
                        ("upload", Some(sc)) => try!(sub_origin_key_upload(ui, sc)),
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        ("pkg", Some(matches)) => {
            match matches.subcommand() {
                ("binlink", Some(m)) => try!(sub_pkg_binlink(ui, m)),
                ("build", Some(m)) => try!(sub_pkg_build(ui, m)),
                ("config", Some(m)) => try!(sub_pkg_config(m)),
                ("exec", Some(m)) => try!(sub_pkg_exec(m, remaining_args)),
                ("export", Some(m)) => try!(sub_pkg_export(ui, m)),
                ("hash", Some(m)) => try!(sub_pkg_hash(m)),
                ("install", Some(m)) => try!(sub_pkg_install(ui, m)),
                ("path", Some(m)) => try!(sub_pkg_path(m)),
                ("provides", Some(m)) => try!(sub_pkg_provides(m)),
                ("search", Some(m)) => try!(sub_pkg_search(m)),
                ("sign", Some(m)) => try!(sub_pkg_sign(ui, m)),
                ("upload", Some(m)) => try!(sub_pkg_upload(ui, m)),
                ("verify", Some(m)) => try!(sub_pkg_verify(ui, m)),
                ("header", Some(m)) => try!(sub_pkg_header(ui, m)),
                _ => unreachable!(),
            }
        }
        ("plan", Some(matches)) => {
            match matches.subcommand() {
                ("init", Some(m)) => try!(sub_plan_init(ui, m)),
                _ => unreachable!(),
            }
        }
        ("ring", Some(matches)) => {
            match matches.subcommand() {
                ("key", Some(m)) => {
                    match m.subcommand() {
                        ("export", Some(sc)) => try!(sub_ring_key_export(sc)),
                        ("import", Some(_)) => try!(sub_ring_key_import(ui)),
                        ("generate", Some(sc)) => try!(sub_ring_key_generate(ui, sc)),
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        ("service", Some(matches)) => {
            match matches.subcommand() {
                ("key", Some(m)) => {
                    match m.subcommand() {
                        ("generate", Some(sc)) => try!(sub_service_key_generate(ui, sc)),
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        ("setup", Some(_)) => try!(sub_cli_setup(ui)),
        ("user", Some(matches)) => {
            match matches.subcommand() {
                ("key", Some(m)) => {
                    match m.subcommand() {
                        ("generate", Some(sc)) => try!(sub_user_key_generate(ui, sc)),
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    };
    Ok(())
}

fn sub_cli_setup(ui: &mut UI) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    init();

    command::cli::setup::start(ui,
                               &default_cache_key_path(fs_root_path),
                               &cache_analytics_path(fs_root_path))
}

fn sub_cli_completers(m: &ArgMatches) -> Result<()> {
    let shell = m.value_of("SHELL").expect("Missing Shell; A shell is required");
    cli::get().gen_completions_to("hab", shell.parse::<Shell>().unwrap(), &mut io::stdout());
    Ok(())
}

fn sub_origin_key_download(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let origin = m.value_of("ORIGIN").unwrap(); // Required via clap
    let revision = m.value_of("REVISION");
    let env_or_default = henv::var(DEPOT_URL_ENVVAR).unwrap_or(DEFAULT_DEPOT_URL.to_string());
    let url = m.value_of("DEPOT_URL").unwrap_or(&env_or_default);

    command::origin::key::download::start(ui,
                                          &url,
                                          &origin,
                                          revision,
                                          &default_cache_key_path(fs_root_path))
}

fn sub_origin_key_export(m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let origin = m.value_of("ORIGIN").unwrap(); // Required via clap
    let pair_type = try!(PairType::from_str(m.value_of("PAIR_TYPE").unwrap()));
    init();

    command::origin::key::export::start(origin, pair_type, &default_cache_key_path(fs_root_path))
}

fn sub_origin_key_generate(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let origin = try!(origin_param_or_env(&m));
    init();

    command::origin::key::generate::start(ui, &origin, &default_cache_key_path(fs_root_path))
}

fn sub_origin_key_import(ui: &mut UI) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let mut content = String::new();
    try!(io::stdin().read_to_string(&mut content));
    init();

    command::origin::key::import::start(ui, &content, &default_cache_key_path(fs_root_path))
}

fn sub_origin_key_upload(ui: &mut UI, m: &ArgMatches) -> Result<()> {

    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));

    let env_or_default = henv::var(DEPOT_URL_ENVVAR).unwrap_or(DEFAULT_DEPOT_URL.to_string());
    let url = m.value_of("DEPOT_URL").unwrap_or(&env_or_default);
    let token = try!(auth_token_param_or_env(&m));

    init();

    if m.is_present("ORIGIN") {
        let origin = m.value_of("ORIGIN").unwrap(); // Required via clap
        // you can either specify files, or infer the latest key names
        let with_secret = m.is_present("WITH_SECRET");
        command::origin::key::upload_latest::start(ui,
                                                   url,
                                                   &token,
                                                   origin,
                                                   with_secret,
                                                   &default_cache_key_path(fs_root_path))
    } else {
        let keyfile = Path::new(m.value_of("PUBLIC_FILE").unwrap());
        let secret_keyfile = m.value_of("SECRET_FILE").map(|f| Path::new(f));
        command::origin::key::upload::start(ui, url, &token, &keyfile, secret_keyfile)
    }
}

fn sub_pkg_binlink(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Path::new(&fs_root);
    let ident = try!(PackageIdent::from_str(m.value_of("PKG_IDENT").unwrap()));
    let binary = m.value_of("BINARY").unwrap(); // Required via clap
    let dest_dir = Path::new(m.value_of("DEST_DIR").unwrap_or(DEFAULT_BINLINK_DIR));

    command::pkg::binlink::start(ui, &ident, &binary, &dest_dir, &fs_root_path)
}

fn sub_pkg_build(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));

    let plan_context = m.value_of("PLAN_CONTEXT").unwrap(); // Required via clap
    let root = m.value_of("HAB_STUDIO_ROOT");
    let src = m.value_of("SRC_PATH");
    let keys_string = match m.values_of("HAB_ORIGIN_KEYS") {
        Some(keys) => {
            init();
            for key in keys.clone() {
                // Validate that all secret keys are present
                let pair = try!(SigKeyPair::get_latest_pair_for(key,
                                &default_cache_key_path(fs_root_path)));
                let _ = pair.secret();
            }
            Some(keys.collect::<Vec<_>>().join(","))
        }
        None => None,
    };
    let keys: Option<&str> = match keys_string.as_ref() {
        Some(s) => Some(s),
        None => None,
    };
    let reuse = m.is_present("REUSE");

    command::pkg::build::start(ui, plan_context, root, src, keys, reuse)
}

fn sub_pkg_config(m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Path::new(&fs_root);
    let ident = try!(PackageIdent::from_str(m.value_of("PKG_IDENT").unwrap()));

    command::pkg::config::start(&ident, &fs_root_path)
}

fn sub_pkg_exec(m: &ArgMatches, cmd_args: Vec<OsString>) -> Result<()> {
    let ident = try!(PackageIdent::from_str(m.value_of("PKG_IDENT").unwrap())); // Required via clap
    let cmd = m.value_of("CMD").unwrap(); // Required via clap

    command::pkg::exec::start(&ident, cmd, cmd_args)
}

fn sub_pkg_export(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let ident = try!(PackageIdent::from_str(m.value_of("PKG_IDENT").unwrap())); // Required via clap
    let format = &m.value_of("FORMAT").unwrap(); // Required via clap
    let export_fmt = try!(command::pkg::export::format_for(ui, &format));
    command::pkg::export::start(ui, &ident, &export_fmt)
}

fn sub_pkg_hash(m: &ArgMatches) -> Result<()> {
    let source = m.value_of("SOURCE").unwrap(); // Required via clap

    init();
    command::pkg::hash::start(&source)
}

fn sub_plan_init(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let name = m.value_of("PKG_NAME").map(|v| v.into());
    let origin = try!(origin_param_or_env(&m));
    let include_callbacks = !m.is_present("NO_CALLBACKS");
    command::plan::init::start(ui, origin, include_callbacks, name)
}

fn sub_pkg_install(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let env_or_default = henv::var(DEPOT_URL_ENVVAR).unwrap_or(DEFAULT_DEPOT_URL.to_string());
    let url = m.value_of("DEPOT_URL").unwrap_or(&env_or_default);
    let ident_or_artifacts = m.values_of("PKG_IDENT_OR_ARTIFACT").unwrap(); // Required via clap
    let ignore_target = if m.is_present("IGNORE_TARGET") {
        true
    } else {
        false
    };
    init();

    for ident_or_artifact in ident_or_artifacts {
        let pkg_ident = try!(common::command::package::install::start(ui,
                                                      url,
                                                      ident_or_artifact,
                                                      PRODUCT,
                                                      VERSION,
                                                      Path::new(&fs_root),
                                                      &cache_artifact_path(fs_root_path),
                                                      ignore_target));
        if m.is_present("BINLINK") {
            let dest_dir = Path::new(m.value_of("DEST_DIR").unwrap_or(DEFAULT_BINLINK_DIR));
            command::pkg::binlink::binlink_all_in_pkg(ui,
                                                      &pkg_ident,
                                                      dest_dir,
                                                      &Path::new(&fs_root))?;
        }
    }
    Ok(())
}

fn sub_pkg_path(m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Path::new(&fs_root);
    let ident = try!(PackageIdent::from_str(m.value_of("PKG_IDENT").unwrap()));

    command::pkg::path::start(&ident, &fs_root_path)
}

fn sub_pkg_provides(m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Path::new(&fs_root);
    let filename = m.value_of("FILE").unwrap(); // Required via clap

    let full_releases = m.is_present("FULL_RELEASES");
    let full_paths = m.is_present("FULL_PATHS");

    command::pkg::provides::start(&filename, &fs_root_path, full_releases, full_paths)
}

fn sub_pkg_search(m: &ArgMatches) -> Result<()> {
    let env_or_default = henv::var(DEPOT_URL_ENVVAR).unwrap_or(DEFAULT_DEPOT_URL.to_string());
    let url = m.value_of("DEPOT_URL").unwrap_or(&env_or_default);
    let search_term = m.value_of("SEARCH_TERM").unwrap(); // Required via clap
    command::pkg::search::start(&search_term, &url)
}

fn sub_pkg_sign(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let src = Path::new(m.value_of("SOURCE").unwrap()); // Required via clap
    let dst = Path::new(m.value_of("DEST").unwrap()); // Required via clap
    init();
    let pair = try!(SigKeyPair::get_latest_pair_for(&try!(origin_param_or_env(&m)),
                                                    &default_cache_key_path(fs_root_path)));

    command::pkg::sign::start(ui, &pair, &src, &dst)
}

fn sub_pkg_upload(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let env_or_default = henv::var(DEPOT_URL_ENVVAR).unwrap_or(DEFAULT_DEPOT_URL.to_string());
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let key_path = cache_key_path(fs_root_path);
    // don't use a pathbuf, as the P generic param for upload::start below is bound to a &str
    let key_path = try!(key_path.to_str().ok_or(Error::CryptoCLI("Invalid key path".to_string())));
    let url = m.value_of("DEPOT_URL").unwrap_or(&env_or_default);
    let token = try!(auth_token_param_or_env(&m));
    let artifact_paths = m.values_of("HART_FILE").unwrap(); // Required via clap
    for artifact_path in artifact_paths {
        try!(command::pkg::upload::start(ui, &url, &token, &artifact_path, &key_path));
    }
    Ok(())
}

fn sub_pkg_verify(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let src = Path::new(m.value_of("SOURCE").unwrap()); // Required via clap
    init();

    command::pkg::verify::start(ui, &src, &default_cache_key_path(fs_root_path))
}

fn sub_pkg_header(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let src = Path::new(m.value_of("SOURCE").unwrap()); // Required via clap
    init();

    command::pkg::header::start(ui, &src)
}

fn sub_ring_key_export(m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let ring = m.value_of("RING").unwrap(); // Required via clap
    init();

    command::ring::key::export::start(ring, &default_cache_key_path(fs_root_path))
}

fn sub_ring_key_generate(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let ring = m.value_of("RING").unwrap(); // Required via clap
    init();

    command::ring::key::generate::start(ui, ring, &default_cache_key_path(fs_root_path))
}

fn sub_ring_key_import(ui: &mut UI) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let mut content = String::new();
    try!(io::stdin().read_to_string(&mut content));
    init();

    command::ring::key::import::start(ui, &content, &default_cache_key_path(fs_root_path))
}

fn sub_service_key_generate(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let org = try!(org_param_or_env(&m));
    let service_group = try!(ServiceGroup::from_str(m.value_of("SERVICE_GROUP").unwrap()));
    init();

    command::service::key::generate::start(ui,
                                           &org,
                                           &service_group,
                                           &default_cache_key_path(fs_root_path))
}

fn sub_user_key_generate(ui: &mut UI, m: &ArgMatches) -> Result<()> {
    let fs_root = henv::var(FS_ROOT_ENVVAR).unwrap_or(FS_ROOT_PATH.to_string());
    let fs_root_path = Some(Path::new(&fs_root));
    let user = m.value_of("USER").unwrap(); // Required via clap
    init();

    command::user::key::generate::start(ui, user, &default_cache_key_path(fs_root_path))
}

fn ui() -> UI {
    let isatty = if henv::var(NONINTERACTIVE_ENVVAR).map(|val| val == "true").unwrap_or(false) {
        Some(false)
    } else {
        None
    };
    let coloring = if henv::var(NOCOLORING_ENVVAR).map(|val| val == "true").unwrap_or(false) {
        Coloring::Never
    } else {
        Coloring::Auto
    };
    UI::default_with(coloring, isatty)
}

fn exec_subcommand_if_called(ui: &mut UI) -> Result<()> {
    let mut args = env::args();
    match (args.nth(1).unwrap_or_default().as_str(), args.next().unwrap_or_default().as_str()) {
        ("butterfly", _) => command::butterfly::start(ui, env::args_os().skip(2).collect()),
        ("apply", _) => {
            let mut args: Vec<OsString> = env::args_os().skip(1).collect();
            args.insert(0, OsString::from("config"));
            command::butterfly::start(ui, args)
        }
        ("config", _) | ("file", _) => {
            command::butterfly::start(ui, env::args_os().skip(1).collect())
        }
        ("stu", _) | ("stud", _) | ("studi", _) | ("studio", _) => {
            command::studio::start(ui, env::args_os().skip(2).collect())
        }
        ("sup", _) => command::sup::start(ui, env::args_os().skip(2).collect()),
        ("start", _) => command::sup::start(ui, env::args_os().skip(1).collect()),
        _ => Ok(()),
    }
}

/// Parse the raw program arguments and split off any arguments that will skip clap's parsing.
///
/// **Note** with the current version of clap there is no clean way to ignore arguments after a
/// certain point, especially if those arguments look like further options and flags.
fn raw_parse_args() -> (Vec<OsString>, Vec<OsString>) {
    let mut args = env::args();
    match (args.nth(1).unwrap_or_default().as_str(), args.next().unwrap_or_default().as_str()) {
        ("pkg", "exec") => {
            if args.by_ref().count() > 2 {
                return (env::args_os().take(5).collect(), env::args_os().skip(5).collect());
            } else {
                (env::args_os().collect(), Vec::new())
            }
        }
        _ => (env::args_os().collect(), Vec::new()),
    }
}

/// Check to see if the user has passed in an AUTH_TOKEN param. If not, check the
/// HAB_AUTH_TOKEN env var. If not, check the CLI config to see if there is a default auth
/// token set. If that's empty too, then error.
fn auth_token_param_or_env(m: &ArgMatches) -> Result<String> {
    match m.value_of("AUTH_TOKEN") {
        Some(o) => Ok(o.to_string()),
        None => {
            match henv::var(AUTH_TOKEN_ENVVAR) {
                Ok(v) => Ok(v),
                Err(_) => {
                    let config = try!(config::load());
                    match config.auth_token {
                        Some(v) => Ok(v),
                        None => return Err(Error::ArgumentError("No auth token specified")),
                    }
                }
            }
        }
    }
}

/// Check to see if the user has passed in an ORIGIN param.  If not, check the HABITAT_ORIGIN env
/// var. If not, check the CLI config to see if there is a default origin set. If that's empty too,
/// then error.
fn origin_param_or_env(m: &ArgMatches) -> Result<String> {
    match m.value_of("ORIGIN") {
        Some(o) => Ok(o.to_string()),
        None => {
            match henv::var(ORIGIN_ENVVAR) {
                Ok(v) => Ok(v),
                Err(_) => {
                    let config = try!(config::load());
                    match config.origin {
                        Some(v) => Ok(v),
                        None => return Err(Error::CryptoCLI("No origin specified".to_string())),
                    }
                }
            }
        }
    }
}

/// Check to see if the user has passed in an ORG param.
/// If not, check the HABITAT_ORG env var. If that's
/// empty too, then error.
fn org_param_or_env(m: &ArgMatches) -> Result<String> {
    match m.value_of("ORG") {
        Some(o) => Ok(o.to_string()),
        None => {
            match henv::var(HABITAT_ORG_ENVVAR) {
                Ok(v) => Ok(v),
                Err(_) => return Err(Error::CryptoCLI("No organization specified".to_string())),
            }
        }
    }
}
