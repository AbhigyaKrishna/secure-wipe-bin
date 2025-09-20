use anyhow::Result;
use crossterm::{
    style::{Color, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use std::{
    io::{self, Write},
    path::Path,
};

pub fn confirm_wipe(path: &Path, demo_mode: bool) -> Result<bool> {
    if demo_mode {
        return Ok(true);
    }

    io::stdout().execute(SetForegroundColor(Color::Red))?;
    println!("WARNING: This will PERMANENTLY destroy all data on:");
    println!("   {}", path.display());
    println!("This operation CANNOT be undone!");
    io::stdout().execute(ResetColor)?;
    println!();
    print!("Type 'WIPE' to confirm: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim() == "WIPE")
}