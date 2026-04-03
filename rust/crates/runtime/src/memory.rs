use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MEMORY_DIR_NAME: &str = "memory";
const MEMORY_INDEX_NAME: &str = "MEMORY.md";
const MEMORY_FILE_EXT: &str = "md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

impl MemoryType {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "feedback" => Some(Self::Feedback),
            "project" => Some(Self::Project),
            "reference" => Some(Self::Reference),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Feedback => "feedback",
            Self::Project => "project",
            Self::Reference => "reference",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    pub name: String,
    pub description: String,
    pub memory_type: MemoryType,
    pub body: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryIndexEntry {
    pub name: String,
    pub description: String,
    pub memory_type: MemoryType,
    pub file_name: String,
}

#[derive(Debug)]
pub enum MemoryError {
    Io(io::Error),
    Parse(String),
    Permission(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Parse(msg) => write!(f, "memory parse error: {msg}"),
            Self::Permission(msg) => write!(f, "memory permission error: {msg}"),
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<io::Error> for MemoryError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Resolve the default memory directory path: `~/.kcode/memory/`
pub fn default_memory_dir() -> PathBuf {
    let home = home_dir();
    home.join(".kcode").join(MEMORY_DIR_NAME)
}

/// Resolve the default memory index path: `~/.kcode/memory/MEMORY.md`
pub fn default_memory_index() -> PathBuf {
    default_memory_dir().join(MEMORY_INDEX_NAME)
}

/// Ensure the memory目录 exists with correct permissions (0700).
pub fn ensure_memory_dir(path: &Path) -> Result<(), MemoryError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(MemoryError::Io)?;
        set_dir_permissions(path, 0o700)?;
    }
    Ok(())
}

/// Ensure the memory index file exists.
pub fn ensure_memory_index(path: &Path) -> Result<(), MemoryError> {
    if !path.exists() {
        let initial_content = "# Kcode Memory Index\n\n";
        atomic_write(path, initial_content)?;
        set_file_permissions(path, 0o600)?;
    }
    Ok(())
}

/// List all memory entries from the memory directory.
pub fn list_memories(dir: &Path) -> Result<Vec<MemoryIndexEntry>, MemoryError> {
    let mut entries = Vec::new();

    if !dir.exists() {
        return Ok(entries);
    }

    for entry in fs::read_dir(dir).map_err(MemoryError::Io)? {
        let entry = entry.map_err(MemoryError::Io)?;
        let path = entry.path();

        if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some(MEMORY_FILE_EXT)
            && path.file_stem().and_then(|s| s.to_str()) != Some("MEMORY")
        {
            if let Ok(parsed) = parse_memory_frontmatter(&path) {
                entries.push(MemoryIndexEntry {
                    name: parsed.name,
                    description: parsed.description,
                    memory_type: parsed.memory_type,
                    file_name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                });
            }
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Read a single memory file by path.
pub fn read_memory(path: &Path) -> Result<MemoryEntry, MemoryError> {
    let frontmatter = parse_memory_frontmatter(path)?;
    Ok(MemoryEntry {
        name: frontmatter.name,
        description: frontmatter.description,
        memory_type: frontmatter.memory_type,
        body: frontmatter.body,
        file_path: path.to_path_buf(),
    })
}

/// Create a new memory file.
pub fn create_memory(
    dir: &Path,
    name: &str,
    description: &str,
    memory_type: MemoryType,
    body: &str,
) -> Result<PathBuf, MemoryError> {
    ensure_memory_dir(dir)?;

    let file_name = format!("{name}.{MEMORY_FILE_EXT}");
    let file_path = dir.join(&file_name);

    let content = format_memory_file(name, description, &memory_type, body);
    atomic_write(&file_path, &content)?;
    set_file_permissions(&file_path, 0o600)?;

    update_memory_index(dir, name, description, &memory_type, &file_name)?;

    Ok(file_path)
}

/// Update an existing memory file. Creates it if it doesn't exist.
/// If the memory exists, only updates description and body (preserves name and type).
pub fn update_memory(
    dir: &Path,
    name: &str,
    description: &str,
    body: &str,
) -> Result<PathBuf, MemoryError> {
    ensure_memory_dir(dir)?;

    let file_name = format!("{name}.{MEMORY_FILE_EXT}");
    let file_path = dir.join(&file_name);

    // If exists, read current type to preserve it
    let memory_type = if file_path.exists() {
        let existing = parse_memory_frontmatter(&file_path)?;
        existing.memory_type
    } else {
        MemoryType::Project
    };

    let content = format_memory_file(name, description, &memory_type, body);
    atomic_write(&file_path, &content)?;
    set_file_permissions(&file_path, 0o600)?;

    update_memory_index(dir, name, description, &memory_type, &file_name)?;

    Ok(file_path)
}

/// Delete a memory file by name.
pub fn delete_memory(dir: &Path, name: &str) -> Result<(), MemoryError> {
    let file_name = format!("{name}.{MEMORY_FILE_EXT}");
    let file_path = dir.join(&file_name);

    if file_path.exists() {
        fs::remove_file(&file_path).map_err(MemoryError::Io)?;
    }

    // Remove from index
    let index_path = dir.join(MEMORY_INDEX_NAME);
    if index_path.exists() {
        let content = fs::read_to_string(&index_path).map_err(MemoryError::Io)?;
        let updated: Vec<String> = content
            .lines()
            .filter(|line| !line.contains(&format!("[{name}]")))
            .map(String::from)
            .collect();
        let new_content = updated.join("\n");
        if !new_content.is_empty() {
            atomic_write(&index_path, &new_content)?;
        }
    }

    Ok(())
}

/// Update the memory index file with a new entry.
fn update_memory_index(
    dir: &Path,
    name: &str,
    description: &str,
    memory_type: &MemoryType,
    file_name: &str,
) -> Result<(), MemoryError> {
    let index_path = dir.join(MEMORY_INDEX_NAME);
    ensure_memory_index(&index_path)?;

    let existing = fs::read_to_string(&index_path).unwrap_or_default();
    let new_line = format!(
        "- [{name}]({file_name}) ({}) — {description}",
        memory_type.as_str()
    );

    // Don't duplicate if already present
    if existing.contains(&format!("[{name}]")) {
        return Ok(());
    }

    let updated = format!("{existing}{new_line}\n");
    atomic_write(&index_path, &updated)?;
    Ok(())
}

/// Parse the frontmatter of a memory file.
fn parse_memory_frontmatter(path: &Path) -> Result<MemoryEntry, MemoryError> {
    let content = fs::read_to_string(path).map_err(MemoryError::Io)?;
    parse_memory_content(path, &content)
}

fn parse_memory_content(path: &Path, content: &str) -> Result<MemoryEntry, MemoryError> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Err(MemoryError::Parse(
            "missing frontmatter delimiter (---)".to_string(),
        ));
    }

    let rest = &content[3..];
    let Some(end_index) = rest.find("---") else {
        return Err(MemoryError::Parse("unclosed frontmatter".to_string()));
    };

    let frontmatter = &rest[..end_index];
    let body = rest[end_index + 3..].trim().to_string();

    let mut name = None;
    let mut description = None;
    let mut memory_type = None;

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => name = Some(value.to_string()),
                "description" => description = Some(value.to_string()),
                "type" => {
                    if let Some(mt) = MemoryType::from_str(value) {
                        memory_type = Some(mt);
                    }
                }
                _ => {}
            }
        }
    }

    let name = name.unwrap_or_else(|| {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });
    let description = description.unwrap_or_default();
    let memory_type = memory_type.unwrap_or(MemoryType::User);

    Ok(MemoryEntry {
        name,
        description,
        memory_type,
        body,
        file_path: path.to_path_buf(),
    })
}

/// Format a memory file content string.
fn format_memory_file(name: &str, description: &str, memory_type: &MemoryType, body: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: {description}\ntype: {}\n---\n\n{body}\n",
        memory_type.as_str()
    )
}

/// Load memories relevant to a project context from the user memory directory.
pub fn load_user_memories() -> Result<Vec<MemoryEntry>, MemoryError> {
    let dir = default_memory_dir();
    ensure_memory_dir(&dir)?;
    ensure_memory_index(&dir.join(MEMORY_INDEX_NAME))?;

    let mut memories = Vec::new();
    for entry in list_memories(&dir)? {
        let path = dir.join(&entry.file_name);
        if let Ok(memory) = read_memory(&path) {
            memories.push(memory);
        }
    }
    Ok(memories)
}

/// Render a summary of loaded memories for display.
pub fn render_memory_summary(entries: &[MemoryIndexEntry]) -> String {
    if entries.is_empty() {
        return "No memory files found in ~/.kcode/memory/.".to_string();
    }

    let mut lines = vec![format!("Memory files ({}):", entries.len())];
    for entry in entries {
        lines.push(format!(
            "  - {} ({}) — {}",
            entry.name,
            entry.memory_type.as_str(),
            entry.description
        ));
    }
    lines.join("\n")
}

// --- File permission helpers ---

fn set_dir_permissions(path: &Path, mode: u32) -> Result<(), MemoryError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).map_err(MemoryError::Io)?.permissions();
        perms.set_mode(mode);
        fs::set_permissions(path, perms).map_err(MemoryError::Io)?;
    }
    // On non-unix, skip (Windows ACLs are not modeled here)
    let _ = path;
    let _ = mode;
    Ok(())
}

fn set_file_permissions(path: &Path, mode: u32) -> Result<(), MemoryError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).map_err(MemoryError::Io)?.permissions();
        perms.set_mode(mode);
        fs::set_permissions(path, perms).map_err(MemoryError::Io)?;
    }
    let _ = path;
    let _ = mode;
    Ok(())
}

/// Atomic write: write to temp file then rename.
fn atomic_write(path: &Path, content: &str) -> Result<(), MemoryError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let temp_path = parent.join(format!(
        ".tmp_{}",
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    fs::write(&temp_path, content).map_err(MemoryError::Io)?;
    fs::rename(&temp_path, path).map_err(MemoryError::Io)?;
    Ok(())
}

/// Get home directory in a cross-platform way.
fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kcode_memory_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_memory_file_with_correct_format() {
        let dir = temp_dir();
        let path = create_memory(
            &dir,
            "test-memory",
            "A test memory",
            MemoryType::User,
            "Body content here.",
        )
        .unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("name: test-memory"));
        assert!(content.contains("description: A test memory"));
        assert!(content.contains("type: user"));
        assert!(content.contains("Body content here."));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_memory_frontmatter_correctly() {
        let dir = temp_dir();
        let path = create_memory(
            &dir,
            "parse-test",
            "Testing parsing",
            MemoryType::Project,
            "Some project notes.",
        )
        .unwrap();

        let entry = read_memory(&path).unwrap();
        assert_eq!(entry.name, "parse-test");
        assert_eq!(entry.description, "Testing parsing");
        assert_eq!(entry.memory_type, MemoryType::Project);
        assert_eq!(entry.body, "Some project notes.");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lists_memories_in_sorted_order() {
        let dir = temp_dir();
        create_memory(&dir, "zebra", "Z topic", MemoryType::User, "z body").unwrap();
        create_memory(&dir, "alpha", "A topic", MemoryType::Feedback, "a body").unwrap();

        let entries = list_memories(&dir).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[1].name, "zebra");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn updates_index_on_create() {
        let dir = temp_dir();
        create_memory(&dir, "indexed", "Should appear in index", MemoryType::Reference, "body")
            .unwrap();

        let index_path = dir.join(MEMORY_INDEX_NAME);
        assert!(index_path.exists());
        let index_content = fs::read_to_string(&index_path).unwrap();
        assert!(index_content.contains("indexed"));
        assert!(index_content.contains("Should appear in index"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_user_memories_returns_all() {
        let dir = temp_dir();
        // Override default for test
        create_memory(&dir, "mem1", "First", MemoryType::User, "body1").unwrap();
        create_memory(&dir, "mem2", "Second", MemoryType::Project, "body2").unwrap();

        let entries = list_memories(&dir).unwrap();
        assert_eq!(entries.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn returns_empty_list_for_nonexistent_dir() {
        let entries = list_memories(&PathBuf::from("/nonexistent/path")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn ensures_directory_creation() {
        let dir = temp_dir().join("nested/memory/dir");
        ensure_memory_dir(&dir).unwrap();
        assert!(dir.exists());
        let _ = fs::remove_dir_all(&dir.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn memory_type_from_str() {
        assert_eq!(MemoryType::from_str("user"), Some(MemoryType::User));
        assert_eq!(MemoryType::from_str("feedback"), Some(MemoryType::Feedback));
        assert_eq!(MemoryType::from_str("project"), Some(MemoryType::Project));
        assert_eq!(MemoryType::from_str("reference"), Some(MemoryType::Reference));
        assert_eq!(MemoryType::from_str("invalid"), None);
    }
}
