use git2::{ObjectType, Repository};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct GitFile {
    pub path: String,
    pub status: String, // "staged" | "modified" | "untracked"
}

#[derive(Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub staged: Vec<GitFile>,
    pub modified: Vec<GitFile>,
    pub untracked: Vec<GitFile>,
}

pub fn git_status(repo_path: &str) -> Result<GitStatus, String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD".to_string());

    let mut staged = vec![];
    let mut modified = vec![];
    let mut untracked = vec![];

    let statuses = repo
        .statuses(None)
        .map_err(|e| format!("git status 失败: {}", e))?;

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("?").to_string();
        let s = entry.status();
        // 使用独立 if（非 if/else）——一个文件可以同时有 staged 和 unstaged 修改（git status 的 MM 行）
        if s.is_index_new()
            || s.is_index_modified()
            || s.is_index_deleted()
            || s.is_index_renamed()
            || s.is_index_typechange()
        {
            staged.push(GitFile { path: path.clone(), status: "staged".into() });
        }
        if s.is_wt_new() {
            untracked.push(GitFile { path, status: "untracked".into() });
        } else if s.is_wt_modified()
            || s.is_wt_deleted()
            || s.is_wt_renamed()
            || s.is_wt_typechange()
        {
            modified.push(GitFile { path, status: "modified".into() });
        }
    }

    Ok(GitStatus { branch, staged, modified, untracked })
}

pub fn git_diff(repo_path: &str, file: &str, staged: bool) -> Result<String, String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file);

    let diff = if staged {
        // tree_opt = None 时（初始提交），diff_tree_to_index 对比空树 → 全部文件新增
        let tree_opt = repo.head().ok()
            .and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(tree_opt.as_ref(), None, Some(&mut opts))
    } else {
        repo.diff_index_to_workdir(None, Some(&mut opts))
    }.map_err(|e| format!("diff 失败: {}", e))?;

    let mut buf = vec![];
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => b"+",
            '-' => b"-",
            ' ' => b" ",
            _ => return true,
        };
        buf.extend_from_slice(prefix);
        buf.extend_from_slice(line.content());
        true
    }).map_err(|e| format!("生成 diff 失败: {}", e))?;

    String::from_utf8(buf).map_err(|e| format!("diff 编码错误: {}", e))
}

pub fn git_stage(repo_path: &str, files: &[String]) -> Result<(), String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;
    let mut index = repo.index().map_err(|e| format!("获取 index 失败: {}", e))?;
    for f in files {
        index.add_path(std::path::Path::new(f))
            .map_err(|e| format!("stage {} 失败: {}", f, e))?;
    }
    index.write().map_err(|e| format!("写入 index 失败: {}", e))?;
    Ok(())
}

pub fn git_unstage(repo_path: &str, files: &[String]) -> Result<(), String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;
    let head = repo.head().map_err(|e| format!("获取 HEAD 失败: {}", e))?;
    let head_obj = head.peel(ObjectType::Commit).map_err(|e| format!("获取 HEAD commit 失败: {}", e))?;
    let paths: Vec<&str> = files.iter().map(|f| f.as_str()).collect();
    repo.reset_default(Some(&head_obj), &paths)
        .map_err(|e| format!("unstage 失败: {}", e))?;
    Ok(())
}

pub fn git_commit(repo_path: &str, message: &str, amend: bool) -> Result<String, String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;
    let sig = repo.signature()
        .map_err(|e| format!("获取签名失败（请配置 git user.name/email）: {}", e))?;
    let mut index = repo.index().map_err(|e| format!("获取 index 失败: {}", e))?;
    let tree_id = index.write_tree().map_err(|e| format!("写入 tree 失败: {}", e))?;
    let tree = repo.find_tree(tree_id).map_err(|e| format!("查找 tree 失败: {}", e))?;

    let head = repo.head().ok();
    let head_commit = head.as_ref().and_then(|h| h.peel_to_commit().ok());

    let oid = if amend {
        // amend: 用 HEAD 的父提交作为新提交的父（替换 HEAD）
        let parent_commits: Vec<git2::Commit> = head_commit.as_ref()
            .map(|c| c.parents().collect())
            .unwrap_or_default();
        let parents: Vec<&git2::Commit> = parent_commits.iter().collect();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )
    } else {
        let parents: Vec<&git2::Commit> = head_commit.iter().collect();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )
    }.map_err(|e| format!("提交失败: {}", e))?;

    Ok(oid.to_string())
}

/// 推送当前分支到 origin，依赖系统 git credential helper
pub fn git_push(repo_path: &str) -> Result<(), String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;
    let mut remote = repo.find_remote("origin")
        .map_err(|e| format!("未找到 origin 远程仓库: {}", e))?;
    let head = repo.head().map_err(|e| format!("获取 HEAD 失败: {}", e))?;
    let branch_name = head.shorthand().unwrap_or("main");
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

    let mut callbacks = git2::RemoteCallbacks::new();
    // 依赖系统 git credential helper（git config credential.helper）
    callbacks.push_update_reference(|refname, status| {
        if let Some(msg) = status {
            eprintln!("push {} 失败: {}", refname, msg);
        }
        Ok(())
    });
    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    remote.push(&[&refspec], Some(&mut push_opts))
        .map_err(|e| format!("推送失败: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use git2::Repository as GitRepo;

    /// 在临时目录创建 git 仓库，返回路径（Drop 时自动清理）
    fn init_temp_repo() -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let repo = GitRepo::init(dir.path()).expect("init 仓库失败");
        // CI 环境可能无全局 user.name/email → 仓库级配置兜底
        let mut cfg = repo.config().expect("获取 config 失败");
        let _ = cfg.set_str("user.name", "test");
        let _ = cfg.set_str("user.email", "test@test.com");
        (dir.path().to_path_buf(), dir)
    }

    /// 在仓库中创建文件并写入内容
    fn write_file(repo: &PathBuf, name: &str, content: &str) {
        let file_path = repo.join(name);
        fs::write(&file_path, content).expect("写入文件失败");
    }

    #[test]
    fn status_empty_repo() {
        let (repo, _tmp) = init_temp_repo();
        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        // CI 环境 git init 默认分支可能是任意名称，只要非空即可
        assert!(!status.branch.is_empty());
        assert!(status.staged.is_empty());
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn status_untracked_file() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "new.txt", "hello");
        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(status.untracked[0].path, "new.txt");
        assert_eq!(status.untracked[0].status, "untracked");
    }

    #[test]
    fn stage_and_status() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "new.txt", "hello");
        git_stage(&repo.to_string_lossy(), &["new.txt".into()]).expect("stage 失败");

        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].path, "new.txt");
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn unstage_file() {
        let (repo, _tmp) = init_temp_repo();
        // 先提交一个文件，让 HEAD 存在（unstage 依赖 HEAD）
        write_file(&repo, "base.txt", "base");
        git_stage(&repo.to_string_lossy(), &["base.txt".into()]).expect("stage 失败");
        git_commit(&repo.to_string_lossy(), "initial", false).expect("commit 失败");
        // 再 stage 一个新文件，然后 unstage
        write_file(&repo, "new.txt", "hello");
        git_stage(&repo.to_string_lossy(), &["new.txt".into()]).expect("stage 失败");
        git_unstage(&repo.to_string_lossy(), &["new.txt".into()]).expect("unstage 失败");

        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        assert!(status.staged.is_empty());
        assert_eq!(status.untracked.len(), 1);
    }

    #[test]
    fn commit_creates_commit() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "new.txt", "hello");
        git_stage(&repo.to_string_lossy(), &["new.txt".into()]).expect("stage 失败");
        let oid = git_commit(&repo.to_string_lossy(), "initial", false).expect("commit 失败");
        assert!(!oid.is_empty());

        // commit 后工作区干净
        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        assert!(status.staged.is_empty());
        assert!(status.modified.is_empty());
    }

    #[test]
    fn modified_after_commit() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "f.txt", "v1");
        git_stage(&repo.to_string_lossy(), &["f.txt".into()]).expect("stage 失败");
        git_commit(&repo.to_string_lossy(), "c1", false).expect("commit 失败");

        // 修改文件 → 应出现在 modified
        write_file(&repo, "f.txt", "v2");
        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.modified[0].path, "f.txt");
    }

    #[test]
    fn dual_status_staged_and_modified() {
        // 文件同时有 staged 和 unstaged 修改 → 应同时出现在两个列表
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "f.txt", "v1");
        git_stage(&repo.to_string_lossy(), &["f.txt".into()]).expect("stage 失败");
        git_commit(&repo.to_string_lossy(), "c1", false).expect("commit 失败");

        write_file(&repo, "f.txt", "v2");
        git_stage(&repo.to_string_lossy(), &["f.txt".into()]).expect("stage 失败");

        // 再修改一次（不 stage）→ staged + modified 同时存在
        write_file(&repo, "f.txt", "v3");
        let status = git_status(&repo.to_string_lossy()).expect("status 失败");
        assert!(status.staged.iter().any(|f| f.path == "f.txt"),
            "应出现在 staged 列表");
        assert!(status.modified.iter().any(|f| f.path == "f.txt"),
            "应同时出现在 modified 列表（文件有额外的未暂存修改）");
    }

    #[test]
    fn diff_staged_shows_content() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "f.txt", "hello");
        git_stage(&repo.to_string_lossy(), &["f.txt".into()]).expect("stage 失败");

        let diff = git_diff(&repo.to_string_lossy(), "f.txt", true).expect("diff 失败");
        assert!(diff.contains("hello"), "staged diff 应包含新增内容: {}", diff);
    }

    #[test]
    fn diff_modified_shows_changes() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "f.txt", "v1");
        git_stage(&repo.to_string_lossy(), &["f.txt".into()]).expect("stage 失败");
        git_commit(&repo.to_string_lossy(), "c1", false).expect("commit 失败");

        write_file(&repo, "f.txt", "v2");
        let diff = git_diff(&repo.to_string_lossy(), "f.txt", false).expect("diff 失败");
        assert!(diff.contains("v2"), "modified diff 应包含新内容: {}", diff);
    }

    #[test]
    fn commit_amend_replaces_head() {
        let (repo, _tmp) = init_temp_repo();
        write_file(&repo, "f.txt", "v1");
        git_stage(&repo.to_string_lossy(), &["f.txt".into()]).expect("stage 失败");
        git_commit(&repo.to_string_lossy(), "first", false).expect("commit 失败");

        write_file(&repo, "g.txt", "extra");
        git_stage(&repo.to_string_lossy(), &["g.txt".into()]).expect("stage 失败");
        let oid = git_commit(&repo.to_string_lossy(), "amended", true).expect("amend 失败");

        // 验证 HEAD 指向新 commit
        let repo_obj = GitRepo::open(&repo).expect("open 失败");
        let head_commit = repo_obj.head().expect("HEAD 失败").peel_to_commit().expect("peel 失败");
        assert_eq!(head_commit.id().to_string(), oid);
        assert!(head_commit.message().unwrap_or("").contains("amended"));
        // amend 后应只有一个父提交
        assert_eq!(head_commit.parent_count(), 1);
    }

    #[test]
    fn error_on_non_repo() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let path = dir.path().to_string_lossy().to_string();
        assert!(git_status(&path).is_err());
        assert!(git_diff(&path, "x", false).is_err());
        assert!(git_stage(&path, &["x".into()]).is_err());
        assert!(git_commit(&path, "msg", false).is_err());
    }
}
