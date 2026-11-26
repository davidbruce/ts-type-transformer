// Machine-readable metadata emitted alongside generated bindings (`tsrs-meta.toml`)
// so cross-package `--ref` resolution never scrapes generated source files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const META_FILE: &str = "tsrs-meta.toml";

/// Exported symbol summary for one generated binding package.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PkgMeta {
    /// Generator target identifier: `"c"`, `"cpp"`, `"zig"`, `"rust"`, `"gleam"`, `"fsharp"`.
    pub target: String,
    /// Package, crate, or F# root namespace name.
    pub package: String,
    /// Optional entry filename relative to the package directory (e.g. `"dom.h"`, `"Dom.fsproj"`).
    pub entry: Option<String>,
    /// Exported top-level type / symbol names (C `all_names`, C++ `types`, Zig `types`, Gleam `classes`).
    pub types: Vec<String>,
    /// Gleam classes/interfaces whose subtyping phantom parameter `(sub)` is kept (base classes with subclasses).
    pub gleam_open: Vec<String>,
    /// C handle struct names defined by this package (not external opaque stubs).
    pub structs: Vec<String>,
    /// C external opaque handle struct names emitted as stubs in this package.
    pub external_structs: Vec<String>,
    /// Original unprefixed TS type name -> emitted C struct name (when renamed/prefixed).
    pub c_structs: BTreeMap<String, String>,
    /// C base struct -> subtype struct names defined in this package.
    pub c_subtypes: BTreeMap<String, Vec<String>>,
    /// Rust exported top-level types: `name -> (arity, is_handle)`.
    pub rust_types: BTreeMap<String, (usize, bool)>,
    /// F# exported top-level types: `name -> (arity, is_interface, abbrev_target)`.
    pub fsharp_types: BTreeMap<String, (usize, bool, Option<String>)>,
}

/// Loaded `tsrs-meta.toml` plus resolved package directory and relative path from the consumer.
pub struct LoadedMeta {
    pub meta: PkgMeta,
    pub root_dir: PathBuf,
    pub rel_dir: String,
}

fn toml_str(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

fn sorted_unique(items: &[String]) -> Vec<String> {
    let mut out = items.to_vec();
    out.sort();
    out.dedup();
    out
}

fn write_str_list(out: &mut String, key: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let vals: Vec<String> = sorted_unique(items).into_iter().map(|s| toml_str(&s)).collect();
    *out += &format!("{key} = [{}]\n", vals.join(", "));
}

fn get_str<'a>(tbl: &'a toml::Table, key: &str, path: &Path) -> Result<&'a str, String> {
    tbl.get(key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{}: missing or invalid string key `{key}`", path.display()))
}

fn get_opt_str_list(tbl: &toml::Table, key: &str, path: &Path) -> Result<Vec<String>, String> {
    let Some(val) = tbl.get(key) else {
        return Ok(Vec::new());
    };
    let arr = val
        .as_array()
        .ok_or_else(|| format!("{}: key `{key}` must be an array", path.display()))?;
    arr.iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{}: elements of `{key}` must be strings", path.display()))
        })
        .collect()
}

fn get_usize(tbl: &toml::Table, key: &str, entry_name: &str, path: &Path) -> Result<usize, String> {
    let n = tbl
        .get(key)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| format!("{}: `{entry_name}.{key}` must be an integer", path.display()))?;
    usize::try_from(n)
        .map_err(|_| format!("{}: `{entry_name}.{key}` must be non-negative", path.display()))
}

fn get_bool(tbl: &toml::Table, key: &str, entry_name: &str, path: &Path) -> Result<bool, String> {
    tbl.get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| format!("{}: `{entry_name}.{key}` must be a bool", path.display()))
}

impl PkgMeta {
    pub fn to_toml(&self) -> String {
        let mut out = format!(
            "target = {}\npackage = {}\n",
            toml_str(&self.target),
            toml_str(&self.package),
        );
        if let Some(entry) = &self.entry {
            out += &format!("entry = {}\n", toml_str(entry));
        }
        write_str_list(&mut out, "types", &self.types);
        write_str_list(&mut out, "gleam_open", &self.gleam_open);
        write_str_list(&mut out, "structs", &self.structs);
        write_str_list(&mut out, "external_structs", &self.external_structs);
        if !self.c_structs.is_empty() {
            out += "\n[c_structs]\n";
            for (orig, emitted) in &self.c_structs {
                out += &format!("{} = {}\n", toml_str(orig), toml_str(emitted));
            }
        }
        if !self.c_subtypes.is_empty() {
            out += "\n[c_subtypes]\n";
            for (base, subs) in &self.c_subtypes {
                write_str_list(&mut out, &toml_str(base), subs);
            }
        }
        if !self.rust_types.is_empty() {
            out += "\n[rust_types]\n";
            for (name, &(arity, handle)) in &self.rust_types {
                out += &format!("{} = {{ arity = {arity}, handle = {handle} }}\n", toml_str(name));
            }
        }
        if !self.fsharp_types.is_empty() {
            out += "\n[fsharp_types]\n";
            for (name, (arity, iface, abbrev)) in &self.fsharp_types {
                match abbrev {
                    Some(target) => {
                        out += &format!(
                            "{} = {{ arity = {arity}, interface = {iface}, abbrev = {} }}\n",
                            toml_str(name),
                            toml_str(target),
                        );
                    }
                    None => {
                        out += &format!(
                            "{} = {{ arity = {arity}, interface = {iface} }}\n",
                            toml_str(name),
                        );
                    }
                }
            }
        }
        out
    }

    pub fn from_toml(src: &str, path: &Path, expected_target: &str) -> Result<Self, String> {
        let root: toml::Table = src
            .parse()
            .map_err(|e| format!("{}: invalid {META_FILE}: {e}", path.display()))?;
        let target = get_str(&root, "target", path)?.to_string();
        if target != expected_target {
            return Err(format!(
                "{}: expected target {:?}, found {:?}",
                path.display(),
                expected_target,
                target
            ));
        }
        let package = get_str(&root, "package", path)?.to_string();
        let entry = match root.get("entry") {
            Some(v) => Some(
                v.as_str()
                    .ok_or_else(|| format!("{}: `entry` must be a string", path.display()))?
                    .to_string(),
            ),
            None => None,
        };
        let types = get_opt_str_list(&root, "types", path)?;
        let gleam_open = get_opt_str_list(&root, "gleam_open", path)?;
        let structs = get_opt_str_list(&root, "structs", path)?;
        let external_structs = get_opt_str_list(&root, "external_structs", path)?;

        let mut c_structs = BTreeMap::new();
        if let Some(val) = root.get("c_structs") {
            let tbl = val
                .as_table()
                .ok_or_else(|| format!("{}: `c_structs` must be a table", path.display()))?;
            for (orig, v) in tbl {
                let emitted = v
                    .as_str()
                    .ok_or_else(|| format!("{}: `c_structs.{orig}` must be a string", path.display()))?
                    .to_string();
                c_structs.insert(orig.clone(), emitted);
            }
        }

        let mut c_subtypes = BTreeMap::new();
        if let Some(val) = root.get("c_subtypes") {
            let tbl = val
                .as_table()
                .ok_or_else(|| format!("{}: `c_subtypes` must be a table", path.display()))?;
            for (base, _) in tbl {
                c_subtypes.insert(base.clone(), get_opt_str_list(tbl, base, path)?);
            }
        }

        let mut rust_types = BTreeMap::new();
        if let Some(val) = root.get("rust_types") {
            let tbl = val
                .as_table()
                .ok_or_else(|| format!("{}: `rust_types` must be a table", path.display()))?;
            for (name, v) in tbl {
                let entry_tbl = v
                    .as_table()
                    .ok_or_else(|| format!("{}: `rust_types.{name}` must be a table", path.display()))?;
                let arity = get_usize(entry_tbl, "arity", name, path)?;
                let handle = get_bool(entry_tbl, "handle", name, path)?;
                rust_types.insert(name.clone(), (arity, handle));
            }
        }

        let mut fsharp_types = BTreeMap::new();
        if let Some(val) = root.get("fsharp_types") {
            let tbl = val
                .as_table()
                .ok_or_else(|| format!("{}: `fsharp_types` must be a table", path.display()))?;
            for (name, v) in tbl {
                let entry_tbl = v
                    .as_table()
                    .ok_or_else(|| format!("{}: `fsharp_types.{name}` must be a table", path.display()))?;
                let arity = get_usize(entry_tbl, "arity", name, path)?;
                let iface = get_bool(entry_tbl, "interface", name, path)?;
                let abbrev = match entry_tbl.get("abbrev") {
                    Some(a) => Some(
                        a.as_str()
                            .ok_or_else(|| format!("{}: `{name}.abbrev` must be a string", path.display()))?
                            .to_string(),
                    ),
                    None => None,
                };
                fsharp_types.insert(name.clone(), (arity, iface, abbrev));
            }
        }

        Ok(PkgMeta {
            target,
            package,
            entry,
            types,
            gleam_open,
            structs,
            external_structs,
            c_structs,
            c_subtypes,
            rust_types,
            fsharp_types,
        })
    }
}

/// Loads `tsrs-meta.toml` from `dir_or_file` (directory or file inside it) and resolves
/// its relative path from `from_dir`.
pub fn load(dir_or_file: &Path, from_dir: &Path, expected_target: &str) -> Result<LoadedMeta, String> {
    let root_dir = if dir_or_file.is_dir() {
        dir_or_file.to_path_buf()
    } else if dir_or_file.is_file()
        || dir_or_file
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| matches!(ext, "toml" | "h" | "hpp" | "zig" | "fsproj"))
    {
        dir_or_file.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
    } else {
        dir_or_file.to_path_buf()
    };
    let meta_path = root_dir.join(META_FILE);
    let src = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let meta = PkgMeta::from_toml(&src, &meta_path, expected_target)?;
    let rel_dir = super::relative_path(from_dir, &root_dir);
    Ok(LoadedMeta {
        meta,
        root_dir,
        rel_dir,
    })
}
