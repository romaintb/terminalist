use crate::{
    icons::IconService,
    sync::SyncService,
    todoist::{LabelDisplay, ProjectDisplay, SectionDisplay},
};

pub struct AppContext {
    pub sync_service: SyncService,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub sections: Vec<SectionDisplay>,
    pub icons: IconService,
}

impl AppContext {
    pub fn new(sync_service: SyncService) -> Self {
        Self {
            sync_service,
            projects: Vec::new(),
            labels: Vec::new(),
            sections: Vec::new(),
            icons: IconService::default(),
        }
    }
}
