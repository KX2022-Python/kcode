use std::fs;
use std::path::Path;

use plugins::{PluginManager, PluginManagerConfig};

use crate::test_support::{temp_dir, write_bundled_plugin, write_external_plugin, write_skill};
use crate::{
    handle_plugins_slash_command, install_skill_into, load_skills_from_roots,
    render_skill_install_report, render_skills_report, DefinitionSource, SkillOrigin, SkillRoot,
};

#[test]
fn installs_skill_into_user_registry_and_preserves_nested_files() {
    let workspace = temp_dir("skills-install-workspace");
    let source_root = workspace.join("source").join("help");
    let install_root = temp_dir("skills-install-root");
    write_skill(
        source_root.parent().expect("parent"),
        "help",
        "Helpful skill",
    );
    let script_dir = source_root.join("scripts");
    fs::create_dir_all(&script_dir).expect("script dir");
    fs::write(script_dir.join("run.sh"), "#!/bin/sh\necho help\n").expect("write script");

    let installed = install_skill_into(
        source_root.to_str().expect("utf8 skill path"),
        &workspace,
        &install_root,
    )
    .expect("skill should install");

    assert_eq!(installed.invocation_name, "help");
    assert_eq!(installed.display_name.as_deref(), Some("help"));
    assert!(installed.installed_path.ends_with(Path::new("help")));
    assert!(installed.installed_path.join("SKILL.md").is_file());
    assert!(installed
        .installed_path
        .join("scripts")
        .join("run.sh")
        .is_file());

    let report = render_skill_install_report(&installed);
    assert!(report.contains("Result           installed help"));
    assert!(report.contains("Invoke as        $help"));
    assert!(report.contains(&install_root.display().to_string()));

    let roots = vec![SkillRoot {
        source: DefinitionSource::UserCodexHome,
        path: install_root.clone(),
        origin: SkillOrigin::SkillsDir,
    }];
    let listed = render_skills_report(
        &load_skills_from_roots(&roots).expect("installed skills should load"),
    );
    assert!(listed.contains("User ($CODEX_HOME):"));
    assert!(listed.contains("help · Helpful skill"));

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(install_root);
}

#[test]
fn installs_plugin_from_path_and_lists_it() {
    let config_home = temp_dir("home");
    let source_root = temp_dir("source");
    write_external_plugin(&source_root, "demo", "1.0.0");

    let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
    let install = handle_plugins_slash_command(
        Some("install"),
        Some(source_root.to_str().expect("utf8 path")),
        &mut manager,
    )
    .expect("install command should succeed");
    let list =
        handle_plugins_slash_command(Some("list"), None, &mut manager).expect("list command");

    assert!(install.reload_runtime);
    assert!(install.message.contains("installed demo@external"));
    assert!(install.message.contains("Status           enabled"));
    assert!(!list.reload_runtime);
    assert!(list.message.contains("demo"));
    assert!(list.message.contains("v1.0.0"));
    assert!(list.message.contains("enabled"));

    let _ = fs::remove_dir_all(config_home);
    let _ = fs::remove_dir_all(source_root);
}

#[test]
fn enables_and_disables_plugin_by_name() {
    let config_home = temp_dir("toggle-home");
    let source_root = temp_dir("toggle-source");
    write_external_plugin(&source_root, "demo", "1.0.0");

    let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
    handle_plugins_slash_command(
        Some("install"),
        Some(source_root.to_str().expect("utf8 path")),
        &mut manager,
    )
    .expect("install command should succeed");

    let disable =
        handle_plugins_slash_command(Some("disable"), Some("demo"), &mut manager).expect("disable");
    let enable =
        handle_plugins_slash_command(Some("enable"), Some("demo"), &mut manager).expect("enable");
    let list =
        handle_plugins_slash_command(Some("list"), None, &mut manager).expect("list command");

    assert!(disable.reload_runtime);
    assert!(disable.message.contains("disabled demo@external"));
    assert!(enable.reload_runtime);
    assert!(enable.message.contains("enabled demo@external"));
    assert!(list.message.contains("demo"));
    assert!(list.message.contains("enabled"));

    let _ = fs::remove_dir_all(config_home);
    let _ = fs::remove_dir_all(source_root);
}

#[test]
fn lists_auto_installed_bundled_plugins_with_status() {
    let config_home = temp_dir("bundled-home");
    let bundled_root = temp_dir("bundled-root");
    let bundled_plugin = bundled_root.join("starter");
    write_bundled_plugin(&bundled_plugin, "starter", "0.1.0", false);

    let mut config = PluginManagerConfig::new(&config_home);
    config.bundled_root = Some(bundled_root.clone());
    let mut manager = PluginManager::new(config);

    let list =
        handle_plugins_slash_command(Some("list"), None, &mut manager).expect("list command");
    assert!(!list.reload_runtime);
    assert!(list.message.contains("starter"));
    assert!(list.message.contains("v0.1.0"));
    assert!(list.message.contains("disabled"));

    let _ = fs::remove_dir_all(config_home);
    let _ = fs::remove_dir_all(bundled_root);
}
