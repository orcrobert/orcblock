use orcblock_core::{ListId, ListName};
use orcblock_ipc::Status;

/// Finds the list `name` names, ignoring case. Without a name, the only
/// list is meant; with several lists the user has to say which.
pub(crate) fn list(status: &Status, name: Option<&str>) -> Result<ListId, String> {
    match name {
        Some(name) => {
            let wanted = ListName::parse(name).ok();
            status
                .lists
                .iter()
                .find(|list| wanted.as_ref().is_some_and(|w| list.name.same_as(w)))
                .map(|list| list.id)
                .ok_or_else(|| format!("No list named {name}; the lists are {}", names(status)))
        }
        None => match status.lists.as_slice() {
            [only] => Ok(only.id),
            _ => Err(format!(
                "Say which list with --list; the lists are {}",
                names(status)
            )),
        },
    }
}

/// The lists a block covers: the named ones, or every list.
pub(crate) fn lists(status: &Status, names: &[String]) -> Result<Vec<ListId>, String> {
    if names.is_empty() {
        return Ok(status.lists.iter().map(|list| list.id).collect());
    }
    names.iter().map(|name| list(status, Some(name))).collect()
}

fn names(status: &Status) -> String {
    status
        .lists
        .iter()
        .map(|list| list.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
