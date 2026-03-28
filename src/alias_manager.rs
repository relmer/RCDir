// src/alias_manager.rs — Top-level orchestrator for --set/--get/--remove-aliases

use crate::command_line::CommandLine;
use crate::console::Console;
use crate::ehm::AppError;





////////////////////////////////////////////////////////////////////////////////
//
//  run
//
//  Dispatch to the appropriate alias sub-command based on parsed switches.
//
////////////////////////////////////////////////////////////////////////////////

pub fn run (cmd: &CommandLine, console: &mut Console) -> Result<(), AppError> {
    if cmd.set_aliases {
        set_aliases (console, cmd.what_if)?;
    } else if cmd.get_aliases {
        get_aliases (console)?;
    } else if cmd.remove_aliases {
        remove_aliases (console, cmd.what_if)?;
    }

    console.flush()?;
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  set_aliases
//
//  Interactive wizard for configuring PowerShell aliases.
//
////////////////////////////////////////////////////////////////////////////////

fn set_aliases (_console: &mut Console, _what_if: bool) -> Result<(), AppError> {
    // TODO: Phase 3 implementation
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_aliases
//
//  Non-interactive display of currently configured aliases.
//
////////////////////////////////////////////////////////////////////////////////

fn get_aliases (_console: &mut Console) -> Result<(), AppError> {
    // TODO: Phase 4 implementation
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  remove_aliases
//
//  Interactive wizard for removing PowerShell aliases.
//
////////////////////////////////////////////////////////////////////////////////

fn remove_aliases (_console: &mut Console, _what_if: bool) -> Result<(), AppError> {
    // TODO: Phase 6 implementation
    Ok(())
}
