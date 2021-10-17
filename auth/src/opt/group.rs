use clap::Parser;

use super::*;

#[derive(Parser)]
#[clap()]
pub struct OptsDomain {
    #[clap(subcommand)]
    pub action: GroupAction,
}

#[derive(Parser)]
pub enum GroupAction {
    /// Creates a new group
    #[clap()]
    Create(CreateGroup),
    /// Removes the existing group
    #[clap()]
    RemoveGroup(GroupRemove),
    /// Adds another user to an existing group
    #[clap()]
    AddUser(GroupAddUser),
    /// Removes a user from an existing group
    #[clap()]
    RemoveUser(GroupRemoveUser),
    /// Display the details about a particular group (token is required to see role membership)
    #[clap()]
    Details(GroupDetails),
}