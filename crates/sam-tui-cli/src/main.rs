//! Native front-end for the Developer Sam TUI. Runs the same
//! backend-agnostic [`sam_tui::view`] component tree that powers
//! developersam.com, so behavior can be exercised in a real terminal.

use anyhow::Result;
use crossterm::tty::IsTty;
use iocraft::ElementExt;
use sam_tui::view;

fn main() -> Result<()> {
    if !std::io::stdout().is_tty() {
        eprintln!("dev-sam: refusing to start because stdout is not a tty");
        return Ok(());
    }
    futures::executor::block_on(async {
        view::root_element()
            .fullscreen()
            .enable_mouse_capture()
            .await?;
        Ok::<(), std::io::Error>(())
    })?;
    Ok(())
}
