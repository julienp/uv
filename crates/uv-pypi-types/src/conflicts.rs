use uv_normalize::{ExtraName, GroupName, PackageName};

/// A list of conflicting sets of extras/groups pre-defined by an end user.
///
/// This is useful to force the resolver to fork according to extras that have
/// unavoidable conflicts with each other. (The alternative is that resolution
/// will fail.)
#[derive(
    Debug, Default, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct Conflicts(Vec<ConflictSet>);

impl Conflicts {
    /// Returns no conflicts.
    ///
    /// This results in no effect on resolution.
    pub fn empty() -> Conflicts {
        Conflicts::default()
    }

    /// Push a single set of conflicts.
    pub fn push(&mut self, set: ConflictSet) {
        self.0.push(set);
    }

    /// Returns an iterator over all sets of conflicting sets.
    pub fn iter(&self) -> impl Iterator<Item = &'_ ConflictSet> + '_ {
        self.0.iter()
    }

    /// Returns true if these conflicts contain any set that contains the given
    /// package and extra name pair.
    pub fn contains<'a>(
        &self,
        package: &PackageName,
        kind: impl Into<ConflictKindRef<'a>>,
    ) -> bool {
        let kind = kind.into();
        self.iter().any(|set| set.contains(package, kind))
    }

    /// Returns true if there are no conflicts.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Appends the given conflicts to this one. This drains all sets from the
    /// conflicts given, such that after this call, it is empty.
    pub fn append(&mut self, other: &mut Conflicts) {
        self.0.append(&mut other.0);
    }
}

/// A single set of package-extra pairs that conflict with one another.
///
/// Within each set of conflicts, the resolver should isolate the requirements
/// corresponding to each extra from the requirements of other extras in
/// this set. That is, the resolver should put each set of requirements in a
/// different fork.
///
/// A `TryFrom<Vec<ConflictItem>>` impl may be used to build a set from a
/// sequence. Note though that at least 2 items are required.
#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct ConflictSet(Vec<ConflictItem>);

impl ConflictSet {
    /// Create a pair of items that conflict with one another.
    pub fn pair(item1: ConflictItem, item2: ConflictItem) -> ConflictSet {
        ConflictSet(vec![item1, item2])
    }

    /// Add a new conflicting item to this set.
    pub fn push(&mut self, item: ConflictItem) {
        self.0.push(item);
    }

    /// Returns an iterator over all conflicting items.
    pub fn iter(&self) -> impl Iterator<Item = &'_ ConflictItem> + '_ {
        self.0.iter()
    }

    /// Returns true if this conflicting item contains the given package and
    /// extra name pair.
    pub fn contains<'a>(
        &self,
        package: &PackageName,
        kind: impl Into<ConflictKindRef<'a>>,
    ) -> bool {
        let kind = kind.into();
        self.iter()
            .any(|set| set.package() == package && *set.kind() == kind)
    }
}

impl<'de> serde::Deserialize<'de> for ConflictSet {
    fn deserialize<D>(deserializer: D) -> Result<ConflictSet, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let set = Vec::<ConflictItem>::deserialize(deserializer)?;
        Self::try_from(set).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<Vec<ConflictItem>> for ConflictSet {
    type Error = ConflictError;

    fn try_from(items: Vec<ConflictItem>) -> Result<ConflictSet, ConflictError> {
        match items.len() {
            0 => return Err(ConflictError::ZeroItems),
            1 => return Err(ConflictError::OneItem),
            _ => {}
        }
        Ok(ConflictSet(items))
    }
}

/// A single item in a conflicting set.
///
/// Each item is a pair of a package and a corresponding extra name for that
/// package.
#[derive(
    Debug,
    Clone,
    Eq,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
    schemars::JsonSchema,
)]
#[serde(
    deny_unknown_fields,
    try_from = "ConflictItemWire",
    into = "ConflictItemWire"
)]
pub struct ConflictItem {
    package: PackageName,
    kind: ConflictKind,
}

impl ConflictItem {
    /// Returns the package name of this conflicting item.
    pub fn package(&self) -> &PackageName {
        &self.package
    }

    /// Returns the package-specific conflict.
    ///
    /// i.e., Either an extra or a group name.
    pub fn kind(&self) -> &ConflictKind {
        &self.kind
    }

    /// Returns the extra name of this conflicting item.
    pub fn extra(&self) -> Option<&ExtraName> {
        self.kind.extra()
    }

    /// Returns the group name of this conflicting item.
    pub fn group(&self) -> Option<&GroupName> {
        self.kind.group()
    }

    /// Returns this item as a new type with its fields borrowed.
    pub fn as_ref(&self) -> ConflictItemRef<'_> {
        ConflictItemRef {
            package: self.package(),
            kind: self.kind.as_ref(),
        }
    }
}

impl From<(PackageName, ExtraName)> for ConflictItem {
    fn from((package, extra): (PackageName, ExtraName)) -> ConflictItem {
        let kind = ConflictKind::Extra(extra);
        ConflictItem { package, kind }
    }
}

impl From<(PackageName, GroupName)> for ConflictItem {
    fn from((package, group): (PackageName, GroupName)) -> ConflictItem {
        let kind = ConflictKind::Group(group);
        ConflictItem { package, kind }
    }
}

/// A single item in a conflicting set, by reference.
///
/// Each item is a pair of a package and a corresponding extra name for that
/// package.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct ConflictItemRef<'a> {
    package: &'a PackageName,
    kind: ConflictKindRef<'a>,
}

impl<'a> ConflictItemRef<'a> {
    /// Returns the package name of this conflicting item.
    pub fn package(&self) -> &'a PackageName {
        self.package
    }

    /// Returns the package-specific conflict.
    ///
    /// i.e., Either an extra or a group name.
    pub fn kind(&self) -> ConflictKindRef<'a> {
        self.kind
    }

    /// Returns the extra name of this conflicting item.
    pub fn extra(&self) -> Option<&'a ExtraName> {
        self.kind.extra()
    }

    /// Returns the group name of this conflicting item.
    pub fn group(&self) -> Option<&'a GroupName> {
        self.kind.group()
    }

    /// Converts this borrowed conflicting item to its owned variant.
    pub fn to_owned(&self) -> ConflictItem {
        ConflictItem {
            package: self.package().clone(),
            kind: self.kind.to_owned(),
        }
    }
}

impl<'a> From<(&'a PackageName, &'a ExtraName)> for ConflictItemRef<'a> {
    fn from((package, extra): (&'a PackageName, &'a ExtraName)) -> ConflictItemRef<'a> {
        let kind = ConflictKindRef::Extra(extra);
        ConflictItemRef { package, kind }
    }
}

impl<'a> From<(&'a PackageName, &'a GroupName)> for ConflictItemRef<'a> {
    fn from((package, group): (&'a PackageName, &'a GroupName)) -> ConflictItemRef<'a> {
        let kind = ConflictKindRef::Group(group);
        ConflictItemRef { package, kind }
    }
}

impl<'a> hashbrown::Equivalent<ConflictItem> for ConflictItemRef<'a> {
    fn equivalent(&self, key: &ConflictItem) -> bool {
        key.as_ref() == *self
    }
}

/// The actual conflicting data for a package.
///
/// That is, either an extra or a group name.
#[derive(Debug, Clone, Eq, Hash, PartialEq, PartialOrd, Ord, schemars::JsonSchema)]
pub enum ConflictKind {
    Extra(ExtraName),
    Group(GroupName),
}

impl ConflictKind {
    /// If this conflict corresponds to an extra, then return the
    /// extra name.
    pub fn extra(&self) -> Option<&ExtraName> {
        match *self {
            ConflictKind::Extra(ref extra) => Some(extra),
            ConflictKind::Group(_) => None,
        }
    }

    /// If this conflict corresponds to a group, then return the
    /// group name.
    pub fn group(&self) -> Option<&GroupName> {
        match *self {
            ConflictKind::Group(ref group) => Some(group),
            ConflictKind::Extra(_) => None,
        }
    }

    /// Returns this conflict as a new type with its fields borrowed.
    pub fn as_ref(&self) -> ConflictKindRef<'_> {
        match *self {
            ConflictKind::Extra(ref extra) => ConflictKindRef::Extra(extra),
            ConflictKind::Group(ref group) => ConflictKindRef::Group(group),
        }
    }
}

/// The actual conflicting data for a package, by reference.
///
/// That is, either a borrowed extra name or a borrowed group name.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum ConflictKindRef<'a> {
    Extra(&'a ExtraName),
    Group(&'a GroupName),
}

impl<'a> ConflictKindRef<'a> {
    /// If this conflict corresponds to an extra, then return the
    /// extra name.
    pub fn extra(&self) -> Option<&'a ExtraName> {
        match *self {
            ConflictKindRef::Extra(extra) => Some(extra),
            ConflictKindRef::Group(_) => None,
        }
    }

    /// If this conflict corresponds to a group, then return the
    /// group name.
    pub fn group(&self) -> Option<&'a GroupName> {
        match *self {
            ConflictKindRef::Group(group) => Some(group),
            ConflictKindRef::Extra(_) => None,
        }
    }

    /// Converts this borrowed conflict to its owned variant.
    pub fn to_owned(&self) -> ConflictKind {
        match *self {
            ConflictKindRef::Extra(extra) => ConflictKind::Extra(extra.clone()),
            ConflictKindRef::Group(group) => ConflictKind::Group(group.clone()),
        }
    }
}

impl<'a> From<&'a ExtraName> for ConflictKindRef<'a> {
    fn from(extra: &'a ExtraName) -> ConflictKindRef<'a> {
        ConflictKindRef::Extra(extra)
    }
}

impl<'a> From<&'a GroupName> for ConflictKindRef<'a> {
    fn from(group: &'a GroupName) -> ConflictKindRef<'a> {
        ConflictKindRef::Group(group)
    }
}

impl<'a> PartialEq<ConflictKind> for ConflictKindRef<'a> {
    fn eq(&self, other: &ConflictKind) -> bool {
        other.as_ref() == *self
    }
}

impl<'a> PartialEq<ConflictKindRef<'a>> for ConflictKind {
    fn eq(&self, other: &ConflictKindRef<'a>) -> bool {
        self.as_ref() == *other
    }
}

impl<'a> hashbrown::Equivalent<ConflictKind> for ConflictKindRef<'a> {
    fn equivalent(&self, key: &ConflictKind) -> bool {
        key.as_ref() == *self
    }
}

/// An error that occurs when the given conflicting set is invalid somehow.
#[derive(Debug, thiserror::Error)]
pub enum ConflictError {
    /// An error for when there are zero conflicting items.
    #[error("Each set of conflicts must have at least two entries, but found none")]
    ZeroItems,
    /// An error for when there is one conflicting items.
    #[error("Each set of conflicts must have at least two entries, but found only one")]
    OneItem,
    /// An error that occurs when the `package` field is missing.
    ///
    /// (This is only applicable when deserializing from the lock file.
    /// When deserializing from `pyproject.toml`, the `package` field is
    /// optional.)
    #[error("Expected `package` field in conflicting entry")]
    MissingPackage,
    /// An error that occurs when both `extra` and `group` are missing.
    #[error("Expected `extra` or `group` field in conflicting entry")]
    MissingExtraAndGroup,
    /// An error that occurs when both `extra` and `group` are present.
    #[error("Expected one of `extra` or `group` in conflicting entry, but found both")]
    FoundExtraAndGroup,
}

/// Like [`Conflicts`], but for deserialization in `pyproject.toml`.
///
/// The schema format is different from the in-memory format. Specifically, the
/// schema format does not allow specifying the package name (or will make it
/// optional in the future), where as the in-memory format needs the package
/// name.
///
/// N.B. `Conflicts` is still used for (de)serialization. Specifically, in the
/// lock file, where the package name is required.
#[derive(
    Debug, Default, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct SchemaConflicts(Vec<SchemaConflictSet>);

impl SchemaConflicts {
    /// Convert the public schema "conflicting" type to our internal fully
    /// resolved type. Effectively, this pairs the corresponding package name
    /// with each conflict.
    ///
    /// If a conflict has an explicit package name (written by the end user),
    /// then that takes precedence over the given package name, which is only
    /// used when there is no explicit package name written.
    pub fn to_conflicts_with_package_name(&self, package: &PackageName) -> Conflicts {
        let mut conflicting = Conflicts::empty();
        for tool_uv_set in &self.0 {
            let mut set = vec![];
            for item in &tool_uv_set.0 {
                let package = item.package.clone().unwrap_or_else(|| package.clone());
                set.push(ConflictItem {
                    package: package.clone(),
                    kind: item.kind.clone(),
                });
            }
            // OK because we guarantee that
            // `SchemaConflictingGroupList` is valid and there aren't
            // any new errors that can occur here.
            let set = ConflictSet::try_from(set).unwrap();
            conflicting.push(set);
        }
        conflicting
    }
}

/// Like [`ConflictSet`], but for deserialization in `pyproject.toml`.
///
/// The schema format is different from the in-memory format. Specifically, the
/// schema format does not allow specifying the package name (or will make it
/// optional in the future), where as the in-memory format needs the package
/// name.
#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct SchemaConflictSet(Vec<SchemaConflictItem>);

/// Like [`ConflictItem`], but for deserialization in `pyproject.toml`.
///
/// The schema format is different from the in-memory format. Specifically, the
/// schema format does not allow specifying the package name (or will make it
/// optional in the future), where as the in-memory format needs the package
/// name.
#[derive(
    Debug,
    Clone,
    Eq,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
    schemars::JsonSchema,
)]
#[serde(
    deny_unknown_fields,
    try_from = "ConflictItemWire",
    into = "ConflictItemWire"
)]
pub struct SchemaConflictItem {
    package: Option<PackageName>,
    kind: ConflictKind,
}

impl<'de> serde::Deserialize<'de> for SchemaConflictSet {
    fn deserialize<D>(deserializer: D) -> Result<SchemaConflictSet, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let items = Vec::<SchemaConflictItem>::deserialize(deserializer)?;
        Self::try_from(items).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<Vec<SchemaConflictItem>> for SchemaConflictSet {
    type Error = ConflictError;

    fn try_from(items: Vec<SchemaConflictItem>) -> Result<SchemaConflictSet, ConflictError> {
        match items.len() {
            0 => return Err(ConflictError::ZeroItems),
            1 => return Err(ConflictError::OneItem),
            _ => {}
        }
        Ok(SchemaConflictSet(items))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ConflictItemWire {
    #[serde(default)]
    package: Option<PackageName>,
    #[serde(default)]
    extra: Option<ExtraName>,
    #[serde(default)]
    group: Option<GroupName>,
}

impl TryFrom<ConflictItemWire> for ConflictItem {
    type Error = ConflictError;

    fn try_from(wire: ConflictItemWire) -> Result<ConflictItem, ConflictError> {
        let Some(package) = wire.package else {
            return Err(ConflictError::MissingPackage);
        };
        match (wire.extra, wire.group) {
            (None, None) => Err(ConflictError::MissingExtraAndGroup),
            (Some(_), Some(_)) => Err(ConflictError::FoundExtraAndGroup),
            (Some(extra), None) => Ok(ConflictItem::from((package, extra))),
            (None, Some(group)) => Ok(ConflictItem::from((package, group))),
        }
    }
}

impl From<ConflictItem> for ConflictItemWire {
    fn from(item: ConflictItem) -> ConflictItemWire {
        match item.kind {
            ConflictKind::Extra(extra) => ConflictItemWire {
                package: Some(item.package),
                extra: Some(extra),
                group: None,
            },
            ConflictKind::Group(group) => ConflictItemWire {
                package: Some(item.package),
                extra: None,
                group: Some(group),
            },
        }
    }
}

impl TryFrom<ConflictItemWire> for SchemaConflictItem {
    type Error = ConflictError;

    fn try_from(wire: ConflictItemWire) -> Result<SchemaConflictItem, ConflictError> {
        let package = wire.package;
        match (wire.extra, wire.group) {
            (None, None) => Err(ConflictError::MissingExtraAndGroup),
            (Some(_), Some(_)) => Err(ConflictError::FoundExtraAndGroup),
            (Some(extra), None) => Ok(SchemaConflictItem {
                package,
                kind: ConflictKind::Extra(extra),
            }),
            (None, Some(group)) => Ok(SchemaConflictItem {
                package,
                kind: ConflictKind::Group(group),
            }),
        }
    }
}

impl From<SchemaConflictItem> for ConflictItemWire {
    fn from(item: SchemaConflictItem) -> ConflictItemWire {
        match item.kind {
            ConflictKind::Extra(extra) => ConflictItemWire {
                package: item.package,
                extra: Some(extra),
                group: None,
            },
            ConflictKind::Group(group) => ConflictItemWire {
                package: item.package,
                extra: None,
                group: Some(group),
            },
        }
    }
}