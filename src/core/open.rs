//! Open command arguments for runa.

use std::ffi::{OsStr, OsString};
use std::path::Path;

/// File variables in open commands: the full path, the file name, the parent directory and the file stem.
const FILE_VARIABLES: [&str; 4] = ["{path}", "{name}", "{dir}", "{stem}"];
const RELATIVE_VARIABLES: [&str; 2] = ["{name}", "{stem}"];

const DASH_GUARD: &str = "./";

/// Returns the arguments after the program, with the file variables filled in.
pub(crate) fn expand_args(cmd: &[String], ext_parts: usize, path: &Path) -> Vec<OsString> {
    let rest = cmd.get(1..).unwrap_or_default();
    let has_variable = rest
        .iter()
        .any(|arg| FILE_VARIABLES.iter().any(|fv| arg.contains(fv)));

    if !has_variable {
        let mut args: Vec<OsString> = rest.iter().map(OsString::from).collect();
        args.push(path.as_os_str().to_owned());
        return args;
    }

    let name = path.file_name().unwrap_or_default();
    let values = [
        path.as_os_str(),
        name,
        path.parent().map(Path::as_os_str).unwrap_or_default(),
        strip_extension(name, ext_parts),
    ];

    rest.iter()
        .map(|arg| {
            let expanded = expand_arg(arg, &values);
            if RELATIVE_VARIABLES.contains(&arg.as_str()) {
                guard_dash(expanded)
            } else {
                expanded
            }
        })
        .collect()
}

/// Prefixes a name that starts with a dash with `./`, so it is never read as an option.
fn guard_dash(name: OsString) -> OsString {
    if name.as_encoded_bytes().first() != Some(&b'-') {
        return name;
    }
    let mut guarded = OsString::with_capacity(DASH_GUARD.len() + name.len());
    guarded.push(DASH_GUARD);
    guarded.push(name);
    guarded
}

/// Returns the name without its last count extension
fn strip_extension(name: &OsStr, count: usize) -> &OsStr {
    let mut stem = name;
    for _ in 0..count {
        match Path::new(stem).file_stem() {
            Some(shorter) if shorter.len() < stem.len() => stem = shorter,
            _ => break,
        }
    }

    stem
}

/// Replaces each file variable in the argument with its value. Unknown braces stay as braces.
fn expand_arg(arg: &str, values: &[&OsStr; FILE_VARIABLES.len()]) -> OsString {
    let mut expanded = OsString::with_capacity(arg.len());
    let mut rest = arg;

    while let Some(start) = rest.find('{') {
        expanded.push(&rest[..start]);
        let tail = &rest[start..];
        let found = FILE_VARIABLES
            .iter()
            .zip(values)
            .find(|(fv, _)| tail.starts_with(**fv));

        match found {
            Some((variable, value)) => {
                expanded.push(value);
                rest = &tail[variable.len()..];
            }
            None => {
                expanded.push("{");
                rest = &tail[1..];
            }
        }
    }

    expanded.push(rest);
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn path_is_appended_without_file_variables() {
        let path = Path::new("/x/main.rs");
        assert_eq!(
            expand_args(&cmd(&["nvim", "-p"]), 1, path),
            os_args(&["-p", "/x/main.rs"])
        );
    }

    #[test]
    fn file_variables_replace_the_appended_path() {
        let path = Path::new("/x/release.tar.gz");
        assert_eq!(
            expand_args(&cmd(&["tar", "-xf", "{path}", "-C", "{dir}"]), 2, path),
            os_args(&["-xf", "/x/release.tar.gz", "-C", "/x"])
        );
    }

    #[test]
    fn stem_drops_the_matched_extension() {
        let path = Path::new("/x/Release.tar.gz");
        assert_eq!(
            expand_args(&cmd(&["7z", "x", "-o{stem}", "{name}"]), 2, path),
            os_args(&["x", "-oRelease", "Release.tar.gz"])
        );
    }

    #[test]
    fn stem_without_extension_match_drops_the_last_extension() {
        let path = Path::new("/x/Cargo.toml");
        assert_eq!(
            expand_args(&cmd(&["show", "{stem}"]), 1, path),
            os_args(&["Cargo"])
        );
    }

    #[test]
    fn dash_names_cannot_become_options() {
        let path = Path::new("/x/-rf.rs");
        assert_eq!(
            expand_args(&cmd(&["rm", "{name}", "{stem}"]), 1, path),
            os_args(&["./-rf.rs", "./-rf"])
        );
    }

    #[test]
    fn embedded_dash_names_stay_unchanged() {
        let path = Path::new("/x/-rf.zip");
        assert_eq!(
            expand_args(&cmd(&["7z", "x", "-o{stem}", "{path}"]), 1, path),
            os_args(&["x", "-o-rf", "/x/-rf.zip"])
        );
    }

    #[test]
    fn unknown_braces_stay_literal() {
        let path = Path::new("/x/a.txt");
        assert_eq!(
            expand_args(&cmd(&["run", "{}", "{foo}", "{path}"]), 1, path),
            os_args(&["{}", "{foo}", "/x/a.txt"])
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_is_kept_exact() {
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(OsStr::from_bytes(b"/x/caf\xe9.zip"));
        let expected = vec![
            OsString::from("x"),
            path.as_os_str().to_owned(),
            OsString::from(OsStr::from_bytes(b"-ocaf\xe9")),
        ];
        assert_eq!(
            expand_args(&cmd(&["7z", "x", "{path}", "-o{stem}"]), 1, path),
            expected
        );
    }
}
