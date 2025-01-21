#[cfg(feature = "frontend")]
use std::process::Command;

fn main() {
    #[cfg(feature = "frontend")]
    {
        // Run the frontend build (e.g. npm or yarn)
        // Adjust the below commands according to your actual build setup.
        // You might need `npm install` once if you're not ensuring dependencies are installed.
        let command = if cfg!(target_os = "windows") {
            "npm.cmd"
        } else {
            "npm"
        };
        println!("Installing frontend dependencies");
        let status = Command::new(command)
            .args(["install"])
            .current_dir("frontend")
            .status()
            .expect("Failed to run npm install for frontend");
        if !status.success() {
            panic!("npm build for frontend failed!");
        }
        println!("Building the frontend");
        let status = Command::new(command)
            .args(["run", "build"])
            .current_dir("frontend")
            .status()
            .expect("Failed to run npm build for frontend");
        if !status.success() {
            panic!("npm build for frontend failed!");
        }
    }
}
