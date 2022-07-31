# MarkDown-Aggregator

This project aims to be an aggregator of repositories containing markdown files

## Configuration

Configuration consists of a toml file, every section represents a different repository to be aggregated.<br />
Section name will be used as descriptor, keeping repository data private.

```toml
[My GitHub Repo]
flavour = "github"
token = "my_token"
owner = "my_username"
repo = "my_repo"
branch = "my_branch"

[My GitLab Repo]
flavour = "gitlab"
token = "<optionally_my_token>"
id = my_project_id
branch = "<optionally_my_branch>"
```

### GitHub

On a public repos, a token with `repo:public_repo` scope is necessary.<br />
For private repos, a token with full `repo` scope is necessary.

### GitLab

On a public repos, no token is required.<br />
For private repos, a token with `read_api` scope is necessary.

## Working features

* access to github repositories
* access to gitlab repositories
* support for gitlab pagination
* interface templating
* async file retrieve

## Missing features

* support github API pagination (?)
* default template styling
* file cache (?)
