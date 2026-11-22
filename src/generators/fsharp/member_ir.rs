use std::collections::{HashMap, HashSet};

use super::syntax::{expand, ident, pascal, subst_params};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FsParam {
    pub name: String,
    pub ty: String,
    pub optional: bool,
    pub param_array: bool,
}

impl FsParam {
    pub(super) fn render(&self) -> String {
        if self.param_array {
            format!("[<System.ParamArray>] {}: {}", self.name, self.ty)
        } else if self.optional {
            format!("?{}: {}", self.name, self.ty)
        } else {
            format!("{}: {}", self.name, self.ty)
        }
    }

    pub(super) fn subst(&self, map: &HashMap<String, String>) -> Self {
        Self {
            name: self.name.clone(),
            ty: subst_params(&self.ty, map),
            optional: self.optional,
            param_array: self.param_array,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PropAccessor {
    ReadOnly,
    Get,
    Set,
    GetSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FsMemberKind {
    AbstractMethod { tparams: String, params: Vec<FsParam>, ret: String },
    StaticMethod { tparams: String, params: Vec<FsParam>, ret: String },
    Property { ty: String, accessor: PropAccessor },
    Indexer { key: String, value: String, settable: bool },
}

/// Structured representation of a generated F# member line for overload resolution,
/// conflict checks, type-parameter substitution, and rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FsMember {
    pub indent: String,
    pub attrs: Vec<String>,
    pub name: String,
    pub kind: FsMemberKind,
    pub marks: Vec<String>,
    // Its `///` lines, written before it.
    pub doc: String,
}

#[derive(PartialEq, Eq)]
enum SigKey {
    Params { is_static: bool, name: String, generic: bool, types: Vec<String> },
    Exact(String),
}

impl FsMember {
    fn render_body(&self) -> String {
        match &self.kind {
            FsMemberKind::AbstractMethod { tparams, params, ret } => {
                let sig = if params.is_empty() {
                    "unit".to_string()
                } else {
                    params.iter().map(FsParam::render).collect::<Vec<_>>().join(" * ")
                };
                format!("abstract {}{tparams}: {sig} -> {ret}", self.name)
            }
            FsMemberKind::StaticMethod { tparams, params, ret } => {
                let pstr = params.iter().map(FsParam::render).collect::<Vec<_>>().join(", ");
                format!("static member {}{tparams} ({pstr}): {ret} = jsNative", self.name)
            }
            FsMemberKind::Property { ty, accessor } => {
                let acc = match accessor {
                    PropAccessor::ReadOnly => "",
                    PropAccessor::Get => " with get",
                    PropAccessor::Set => " with set",
                    PropAccessor::GetSet => " with get, set",
                };
                format!("abstract {}: {ty}{acc}", self.name)
            }
            FsMemberKind::Indexer { key, value, settable } => {
                let acc = if *settable { " with get, set" } else { "" };
                format!("abstract {}: key: {key} -> {value}{acc}", self.name)
            }
        }
    }

    pub(super) fn render_unindented(&self) -> String {
        if self.attrs.is_empty() {
            self.render_body()
        } else {
            format!("{} {}", self.attrs.join(" "), self.render_body())
        }
    }

    pub(super) fn render(&self) -> String {
        format!("{}{}    {}\n", self.doc, self.indent, self.render_unindented())
    }

    pub(super) fn subst(&self, map: &HashMap<String, String>) -> Self {
        let kind = match &self.kind {
            FsMemberKind::AbstractMethod { tparams, params, ret } => FsMemberKind::AbstractMethod {
                tparams: subst_params(tparams, map),
                params: params.iter().map(|p| p.subst(map)).collect(),
                ret: subst_params(ret, map),
            },
            FsMemberKind::StaticMethod { tparams, params, ret } => FsMemberKind::StaticMethod {
                tparams: subst_params(tparams, map),
                params: params.iter().map(|p| p.subst(map)).collect(),
                ret: subst_params(ret, map),
            },
            FsMemberKind::Property { ty, accessor } => FsMemberKind::Property {
                ty: subst_params(ty, map),
                accessor: *accessor,
            },
            FsMemberKind::Indexer { key, value, settable } => FsMemberKind::Indexer {
                key: subst_params(key, map),
                value: subst_params(value, map),
                settable: *settable,
            },
        };
        Self {
            indent: self.indent.clone(),
            attrs: self.attrs.clone(),
            name: self.name.clone(),
            kind,
            marks: self.marks.clone(),
            doc: self.doc.clone(),
        }
    }

    pub(super) fn redeclared_key(&self) -> (String, &'static str) {
        let acc = match &self.kind {
            FsMemberKind::Property { accessor: PropAccessor::Get, .. } => "get",
            FsMemberKind::Property { accessor: PropAccessor::Set, .. } => "set",
            _ => "",
        };
        (self.name.clone(), acc)
    }

    fn sig_key(&self, abbrevs: &HashMap<String, String>) -> SigKey {
        let param_types = |params: &[FsParam]| -> Vec<String> {
            params
                .iter()
                .map(|p| {
                    let t = expand(&p.ty, abbrevs);
                    if p.optional { format!("?{t}") } else { t }
                })
                .collect()
        };
        match &self.kind {
            FsMemberKind::AbstractMethod { tparams, params, .. } => SigKey::Params {
                is_static: false,
                name: self.name.clone(),
                generic: !tparams.is_empty(),
                types: if params.is_empty() { vec!["unit".into()] } else { param_types(params) },
            },
            FsMemberKind::StaticMethod { tparams, params, .. } => SigKey::Params {
                is_static: true,
                name: self.name.clone(),
                generic: !tparams.is_empty(),
                types: param_types(params),
            },
            FsMemberKind::Indexer { key, .. } => SigKey::Params {
                is_static: false,
                name: self.name.clone(),
                generic: false,
                types: vec![expand(key, abbrevs)],
            },
            FsMemberKind::Property { .. } => SigKey::Exact(self.render_body()),
        }
    }

    fn head_key(&self) -> Option<(bool, &str, bool)> {
        match &self.kind {
            FsMemberKind::AbstractMethod { tparams, .. } => Some((false, &self.name, !tparams.is_empty())),
            FsMemberKind::StaticMethod { tparams, .. } => Some((true, &self.name, !tparams.is_empty())),
            FsMemberKind::Indexer { .. } => Some((false, &self.name, false)),
            FsMemberKind::Property { .. } => None,
        }
    }

    fn arities(&self, abbrevs: &HashMap<String, String>) -> Option<(Vec<Vec<String>>, bool)> {
        let params = match &self.kind {
            FsMemberKind::AbstractMethod { params, .. } | FsMemberKind::StaticMethod { params, .. } => params.as_slice(),
            FsMemberKind::Indexer { key, .. } => return Some((vec![vec![expand(key, abbrevs)]], false)),
            FsMemberKind::Property { .. } => return None,
        };
        let has_param_array = params.iter().any(|p| p.param_array);
        if params.is_empty() || (params.len() == 1 && !params[0].optional && !params[0].param_array && params[0].ty == "unit") {
            return Some((vec![Vec::new()], has_param_array));
        }
        let types: Vec<String> = params.iter().map(|p| expand(&p.ty, abbrevs)).collect();
        let mut min_req = params.len();
        while min_req > 0 && params[min_req - 1].optional {
            min_req -= 1;
        }
        Some(((min_req..=types.len()).map(|n| types[..n].to_vec()).collect(), has_param_array))
    }

    fn rename(&mut self, to: String) {
        let bare = self.name.trim_matches('`');
        if !self.attrs.iter().any(|a| a.contains("Emit")) {
            let emit = if ident(bare) == bare {
                format!("[<Emit(\"$0.{bare}($1...)\")>]")
            } else {
                format!("[<Emit(\"$0[\\\"{bare}\\\"]($1...)\")>]")
            };
            self.attrs.insert(0, emit);
        }
        self.name = to;
    }
}

// Adds a member. One F# would call as another is there (TypeScript
// overloads apart only in what F# leaves out: a literal option, a
// constraint): the same line is left out. Else each is named for what it
// has that the other has not, `merge` and `mergeDictionary`, `marked` and
// `markedAsync`, the one with nothing of its own keeping the name; where
// neither has, the next free `name_2`. Both call the same JavaScript.
pub(super) fn push_member(members: &mut Vec<FsMember>, mut cand: FsMember, abbrevs: &HashMap<String, String>) {
    let key = cand.sig_key(abbrevs);
    let Some(at) = members.iter().position(|m| m.sig_key(abbrevs) == key) else {
        members.push(cand);
        return;
    };
    if matches!(key, SigKey::Exact(_))
        || expand(&members[at].render_unindented(), abbrevs) == expand(&cand.render_unindented(), abbrevs)
    {
        return;
    }
    let bare = cand.name.trim_matches('`').to_string();
    let own = |a: &[String], b: &[String]| a.iter().filter(|m| !b.contains(m)).map(|m| pascal(m)).collect::<String>();
    let (new_own, old_own) = (own(&cand.marks, &members[at].marks), own(&members[at].marks, &cand.marks));
    let mut taken: HashSet<String> = members.iter().map(|m| m.name.clone()).collect();
    let mut free = |suffix: &str| -> String {
        let base = format!("{bare}{suffix}");
        let n = std::iter::once(ident(&base))
            .chain((2..).map(|n| ident(&format!("{base}_{n}"))))
            .find(|n| !taken.contains(n))
            .unwrap();
        taken.insert(n.clone());
        n
    };
    if !old_own.is_empty() && new_own.is_empty() {
        // The one there has what this has not: it is renamed, this takes the name.
        let renamed = free(&old_own);
        members[at].rename(renamed);
        members.push(cand);
    } else {
        let renamed = free(&new_own);
        cand.rename(renamed);
        members.push(cand);
    }
}

// Whether adding `cand` (an expanded union overload) would collide with or
// create a call-site arity ambiguity against any member already in `members`.
pub(super) fn has_member_conflict(members: &[FsMember], cand: &FsMember, abbrevs: &HashMap<String, String>) -> bool {
    let Some(head) = cand.head_key() else { return true };
    let Some((cand_arities, cand_has_param_array)) = cand.arities(abbrevs) else { return true };
    members.iter().any(|m| {
        if m.head_key() != Some(head) {
            return false;
        }
        let Some((existing, existing_param_array)) = m.arities(abbrevs) else { return true };
        if cand_arities.iter().any(|c| existing.contains(c)) {
            return true;
        }
        // If either side has a trailing ParamArray, also check dropping that last element.
        if cand_has_param_array && cand_arities.last().is_some_and(|c| !c.is_empty() && existing.iter().any(|e| e.as_slice() == &c[..c.len() - 1])) {
            return true;
        }
        if existing_param_array && existing.last().is_some_and(|e| !e.is_empty() && cand_arities.iter().any(|c| c.as_slice() == &e[..e.len() - 1])) {
            return true;
        }
        false
    })
}

// The members without their documentation, for what scans the code for types.
pub(super) fn render_code(members: &[FsMember]) -> String {
    members.iter().map(|m| FsMember { doc: String::new(), ..m.clone() }.render()).collect()
}

pub(super) fn render_members(members: &[FsMember]) -> String {
    members.iter().map(FsMember::render).collect()
}
