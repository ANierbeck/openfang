// Test for agent persistence fix - killed agents should not be restored on startup
use openfang_kernel::OpenFangKernel;
use openfang_types::agent::AgentManifest;
use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::test]
async fn test_killed_agents_not_restored() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let home_dir = temp_dir.path().to_path_buf();
    
    // Create a minimal config
    let config_content = r#"
[default_model]
provider = "test"
model = "test-model"
api_key_env = "TEST_API_KEY"
"#;
    
    std::fs::create_dir_all(home_dir.join("agents")).unwrap();
    std::fs::write(home_dir.join("config.toml"), config_content).unwrap();
    
    // Set environment variable for the test
    std::env::set_var("TEST_API_KEY", "test-key");
    std::env::set_var("OPENFANG_HOME", home_dir.to_str().unwrap());
    
    // Create a simple agent manifest
    let manifest = AgentManifest {
        name: "test-agent".to_string(),
        description: "Test agent".to_string(),
        ..Default::default()
    };
    
    // Boot kernel and spawn agent
    let kernel = OpenFangKernel::boot_with_config(
        openfang_kernel::config::load_config(Some(&home_dir.join("config.toml"))).unwrap()
    ).unwrap();
    
    let agent_id = kernel.spawn_agent(manifest).unwrap();
    
    // Verify agent is running
    let agents_before = kernel.registry.list();
    assert_eq!(agents_before.len(), 1);
    assert_eq!(agents_before[0].name, "test-agent");
    
    // Kill the agent
    kernel.kill_agent(agent_id).unwrap();
    
    // Verify agent is removed from registry
    let agents_after_kill = kernel.registry.list();
    assert_eq!(agents_after_kill.len(), 0);
    
    // Shutdown kernel (this would normally persist agents)
    kernel.shutdown();
    
    // Boot a new kernel to see if killed agent is restored
    let kernel2 = OpenFangKernel::boot_with_config(
        openfang_kernel::config::load_config(Some(&home_dir.join("config.toml"))).unwrap()
    ).unwrap();
    
    // Verify that no agents are restored (the killed agent should not come back)
    let agents_after_restart = kernel2.registry.list();
    assert_eq!(agents_after_restart.len(), 0, "Killed agents should not be restored on startup");
    
    kernel2.shutdown();
    
    // Cleanup
    temp_dir.close().unwrap();
}