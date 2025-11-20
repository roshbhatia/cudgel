use sample_repo::{create_app, Config};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let app = create_app();
    println!("Sample Application v0.1.0");
    println!("Config: {:?}", app.config());
    
    print!("Enter some text to process: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match app.process_input(&input) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    print!("Enter email to validate: ");
    io::stdout().flush()?;
    
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    
    let email = email.trim();
    if app.validate_user_email(email) {
        println!("✓ Valid email: {}", email);
    } else {
        println!("✗ Invalid email: {}", email);
    }
    
    Ok(())
}