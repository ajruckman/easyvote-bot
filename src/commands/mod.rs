mod poll;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use crate::helpers::command_def::{CommandDef, InteractionHandler};

pub const COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: poll::POLL,
        builder: poll::poll_builder,
        handler: |c, i| Box::pin(async move { poll::poll(c, i).await }),
        re_register: true,
        whitelisted_servers: None,
    }
];

static COMMAND_MAP: Lazy<DashMap<String, InteractionHandler>> = Lazy::new(|| {
    let map = DashMap::new();

    for cmd in COMMANDS {
        map.insert(cmd.name.to_string(), cmd.handler);
    }

    map
});

pub fn get_handler(command_name: &str) -> Option<InteractionHandler> {
    COMMAND_MAP
        .get(command_name)
        .as_ref()
        .map(|entry| *entry.value())
}
