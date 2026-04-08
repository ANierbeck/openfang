// Test for agent persistence fix - killed agents should not be restored on startup
use openfang_kernel::OpenFangKernel;
use openfang_types::agent::AgentManifest;
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
    // Use canonical path for Windows compatibility
    let home_dir_str = home_dir.to_str().unwrap();
    std::env::set_var("OPENFANG_HOME", home_dir_str);

    // Create a simple agent manifest
    let manifest = AgentManifest {
        name: "test-agent".to_string(),
        description: "Test agent".to_string(),
        ..Default::default()
    };

    // Boot kernel and spawn agent
    let config_path = home_dir.join("config.toml");
    eprintln!("DEBUG: Loading config from: {:?}", config_path);
    eprintln!("DEBUG: Config exists: {}", config_path.exists());
    
    let kernel = OpenFangKernel::boot_with_config(openfang_kernel::config::load_config(Some(
        &config_path,
    )))
    .unwrap();

    let agent_id = kernel.spawn_agent(manifest).unwrap();

    // Verify agent is running
    let agents_before = kernel.registry.list();
    eprintln!(
        "DEBUG: Agents before kill: {:?}",
        agents_before
            .iter()
            .map(|a| (a.id, a.name.clone(), a.state))
            .collect::<Vec<_>>()
    );
    // Use the returned agent_id directly
    let test_agent_id = agent_id;

    // Kill the test agent
    kernel.kill_agent(test_agent_id).unwrap();

    // Verify test agent is removed from registry (other agents like "assistant" may remain)
    let agents_after_kill = kernel.registry.list();
    let test_agent_after_kill = agents_after_kill.iter().find(|a| a.name == "test-agent");
    assert!(
        test_agent_after_kill.is_none(),
        "test-agent should be removed from registry after kill"
    );

    // Explicitly ensure database operations are complete
    kernel.memory.sync().ok();

    // Shutdown kernel (this would normally persist agents)
    kernel.shutdown();

    // Boot a new kernel to see if killed agent is restored
    let kernel2 = OpenFangKernel::boot_with_config(openfang_kernel::config::load_config(Some(
        &home_dir.join("config.toml"),
    )))
    .unwrap();

    // Verify that the test agent is not restored (other agents like "assistant" may be present)
    let agents_after_restart = kernel2.registry.list();
    eprintln!(
        "DEBUG: Agents after restart: {:?}",
        agents_after_restart
            .iter()
            .map(|a| (a.id, a.name.clone(), a.state))
            .collect::<Vec<_>>()
    );
    let test_agent_after_restart = agents_after_restart.iter().find(|a| a.name == "test-agent");
    assert!(
        test_agent_after_restart.is_none(),
        "Killed test-agent should not be restored on startup. Found agents: {:?}",
        agents_after_restart
            .iter()
            .map(|a| (a.id, a.name.clone()))
            .collect::<Vec<_>>()
    );

    kernel2.shutdown();

    // Cleanup
    // On Windows, ensure proper cleanup by explicitly dropping kernel resources first
    drop(kernel2);
    temp_dir.close().unwrap();
}

#[cfg(windows)]
#[tokio::test]
async fn test_windows_compatibility() {
    // This test ensures basic Windows compatibility
    // Create a simple test to verify path handling works on Windows
    let temp_dir = tempfile::tempdir().unwrap();
    let test_path = temp_dir.path().join("test.txt");
    
    // Test file operations that might fail on Windows
    std::fs::write(&test_path, "test content").unwrap();
    let content = std::fs::read_to_string(&test_path).unwrap();
    assert_eq!(content, "test content");
    
    // Test environment variable handling
    std::env::set_var("TEST_WINDOWS_VAR", "test_value");
    let var_value = std::env::var("TEST_WINDOWS_VAR").unwrap();
    assert_eq!(var_value, "test_value");
    
    temp_dir.close().unwrap();
}
