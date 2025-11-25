use ide_db::{
    base_db::{CrateOrigin, RootQueryDb},
    FileId, FxIndexSet, RootDatabase,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrateInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub root_file_id: FileId,
}

pub(crate) fn fetch_crates(db: &RootDatabase) -> FxIndexSet<CrateInfo> {
    db.all_crates()
        .iter()
        .copied()
        .map(|crate_id| (crate_id.data(db), crate_id.extra_data(db)))
        .filter(|(data, _)| !matches!(data.origin, CrateOrigin::Local { .. }))
        .map(|(data, extra_data)| crate_info(data, extra_data))
        .collect()
}

fn crate_info(
    data: &ide_db::base_db::BuiltCrateData,
    extra_data: &ide_db::base_db::ExtraCrateData,
) -> CrateInfo {
    let crate_name = crate_name(extra_data);
    let version = extra_data.version.clone();
    CrateInfo { name: crate_name, version, root_file_id: data.root_file_id }
}

fn crate_name(data: &ide_db::base_db::ExtraCrateData) -> Option<String> {
    data.display_name.as_ref().map(|it| it.canonical_name().as_str().to_owned())
}
