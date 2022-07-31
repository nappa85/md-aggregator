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

To be aggregated, you must provide an OAuth access token with full `repo` scope. Probably a public repo would need only `repo:public_repo` scope, haven't tested yet.

### GitLab

On a public project, no token is required. For private projects, a token with `read_api` scope is necessary. Probably also `read_repository` scope would be enough, haven't tested yet.

## Working features

* access to github repositories
* access to gitlab repositories
* interface templating
* async file retrieve

## Missing features

* support github API pagination
* support gitlab API pagination
* filter/manage not markdown files
* default template styling
* file cache (?)
