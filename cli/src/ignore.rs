use std::path::Path;

pub struct IgnoreMatcher {
    patterns: Vec<String>,
}

impl IgnoreMatcher {
    pub fn new(patterns: &[String]) -> Self {
        Self {
            patterns: patterns.to_vec(),
        }
    }

    pub fn is_ignored(&self, rel_path: &str) -> bool {
        // 统一使用正斜杠
        let normalized = rel_path.replace('\\', "/");
        let path = Path::new(&normalized);

        for pattern in &self.patterns {
            if pattern.contains('/') {
                // 路径模式：匹配完整相对路径或其前缀
                // 例如 "projects/temp" 匹配 "projects/temp" 及 "projects/temp/foo.txt"
                if Self::matches_pattern(&normalized, pattern) {
                    return true;
                }
                // 也检查路径是否以 pattern/ 开头
                if normalized.starts_with(&format!("{}/", pattern)) {
                    return true;
                }
            } else {
                // 名称模式：匹配路径中任意一个组件
                for component in path.components() {
                    let name = component.as_os_str().to_string_lossy();
                    if Self::matches_pattern(&name, pattern) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn matches_pattern(name: &str, pattern: &str) -> bool {
        // Simple exact match and glob matching
        if pattern.contains('*') {
            Self::glob_match(name, pattern)
        } else {
            name == pattern
        }
    }

    fn glob_match(text: &str, pattern: &str) -> bool {
        // Simple glob: only supports * (match any sequence) and ? (match single char)
        let t_chars: Vec<char> = text.chars().collect();
        let p_chars: Vec<char> = pattern.chars().collect();
        Self::glob_match_recursive(&t_chars, &p_chars)
    }

    fn glob_match_recursive(text: &[char], pattern: &[char]) -> bool {
        if pattern.is_empty() {
            return text.is_empty();
        }
        if pattern[0] == '*' {
            // * matches zero or more characters
            for i in 0..=text.len() {
                if Self::glob_match_recursive(&text[i..], &pattern[1..]) {
                    return true;
                }
            }
            false
        } else if text.is_empty() {
            false
        } else if pattern[0] == '?' || pattern[0] == text[0] {
            Self::glob_match_recursive(&text[1..], &pattern[1..])
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let m = IgnoreMatcher::new(&[".DS_Store".to_string()]);
        assert!(m.is_ignored(".DS_Store"));
        assert!(m.is_ignored("subdir/.DS_Store"));
        assert!(!m.is_ignored("readme.md"));
    }

    #[test]
    fn test_glob_match() {
        let m = IgnoreMatcher::new(&["*.tmp".to_string()]);
        assert!(m.is_ignored("file.tmp"));
        assert!(m.is_ignored("dir/file.tmp"));
        assert!(!m.is_ignored("file.txt"));
    }

    #[test]
    fn test_ssd_syncer_ignored() {
        let m = IgnoreMatcher::new(&[".ssd-syncer".to_string()]);
        assert!(m.is_ignored(".ssd-syncer/snapshots/mac/foo.json"));
        assert!(!m.is_ignored("my-project/main.rs"));
    }

    #[test]
    fn test_path_pattern() {
        // 路径模式：只忽略特定路径下的目录
        let m = IgnoreMatcher::new(&["projects/temp".to_string()]);
        assert!(m.is_ignored("projects/temp"));
        assert!(m.is_ignored("projects/temp/foo.txt"));
        assert!(!m.is_ignored("other/temp"));
        assert!(!m.is_ignored("temp"));
    }

    #[test]
    fn test_name_vs_path_pattern() {
        // 名称模式 "target" 忽略所有叫 target 的
        let m1 = IgnoreMatcher::new(&["target".to_string()]);
        assert!(m1.is_ignored("project-a/target"));
        assert!(m1.is_ignored("project-b/target/debug/main"));

        // 路径模式 "project-a/target" 只忽略特定路径
        let m2 = IgnoreMatcher::new(&["project-a/target".to_string()]);
        assert!(m2.is_ignored("project-a/target"));
        assert!(m2.is_ignored("project-a/target/debug/main"));
        assert!(!m2.is_ignored("project-b/target"));
    }
}
