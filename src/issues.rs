//! Interfaces for accessing and managing issues

// Third party
use serde::Serialize;
use std::collections::BTreeMap;
use url::form_urlencoded;

// Ours
use crate::{Board, Issue, Jira, Result, SearchOptions};

/// Issue options
#[derive(Debug)]
pub struct Issues {
    jira: Jira,
}

#[derive(Serialize, Debug, Clone)]
pub struct Assignee {
    pub name: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct IssueType {
    pub id: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Priority {
    pub id: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Project {
    pub key: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Component {
    pub name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    pub assignee: Assignee,
    pub components: Vec<Component>,
    pub description: String,
    pub environment: String,
    pub issuetype: IssueType,
    pub priority: Priority,
    pub project: Project,
    pub reporter: Assignee,
    pub summary: String,
}

#[derive(Serialize, Debug)]
pub struct CreateIssue {
    pub fields: Fields,
}

#[derive(Debug, Deserialize)]
pub struct CreateResponse {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub url: String,
}

#[derive(Serialize, Debug)]
pub struct EditIssue<T: Serialize> {
    pub fields: BTreeMap<String, T>,
}

#[derive(Deserialize, Debug)]
pub struct IssueResults {
    pub expand: Option<String>,
    #[serde(rename = "maxResults")]
    pub max_results: u64,
    #[serde(rename = "startAt")]
    pub start_at: u64,
    pub total: u64,
    pub issues: Vec<Issue>,
}

impl Issues {
    pub fn new(jira: &Jira) -> Issues {
        Issues { jira: jira.clone() }
    }

    /// Get a single issue
    ///
    /// See this [jira docs](https://docs.atlassian.com/jira-software/REST/latest/#agile/1.0/issue)
    /// for more information
    pub fn get<I>(&self, id: I) -> Result<Issue>
    where
        I: Into<String>,
    {
        self.jira.get("agile", &format!("/issue/{}", id.into()))
    }

    /// Create a new issue
    ///
    /// See this [jira docs](https://docs.atlassian.com/software/jira/docs/api/REST/latest/#api/2/issue-createIssue)
    /// for more information
    pub fn create(&self, data: CreateIssue) -> Result<CreateResponse> {
        self.jira.post("api", "/issue", data)
    }

    /// Edit an issue
    ///
    /// See this [jira docs](https://docs.atlassian.com/software/jira/docs/api/REST/latest/#api/2/issue-editIssue)
    /// for more information
    pub fn edit<I, T>(&self, id: I, data: EditIssue<T>) -> Result<()>
    where
        I: Into<String>,
        T: Serialize,
    {
        self.jira.put("api", &format!("/issue/{}", id.into()), data)
    }

    /// Returns a single page of issue results
    ///
    /// See this [jira docs](https://docs.atlassian.com/jira-software/REST/latest/#agile/1.0/board-getIssuesForBoard)
    /// for more information
    pub fn list(&self, board: &Board, options: &SearchOptions) -> Result<IssueResults> {
        let mut path = vec![format!("/board/{}/issue", board.id)];
        let query_options = options.serialize().unwrap_or_default();
        let query = form_urlencoded::Serializer::new(query_options).finish();

        path.push(query);

        self.jira
            .get::<IssueResults>("agile", path.join("?").as_ref())
    }

    /// Returns a type which may be used to iterate over consecutive pages of results
    ///
    /// See this [jira docs](https://docs.atlassian.com/jira-software/REST/latest/#agile/1.0/board-getIssuesForBoard)
    /// for more information
    pub fn iter<'a>(&self, board: &'a Board, options: &'a SearchOptions) -> Result<IssuesIter<'a>> {
        IssuesIter::new(board, options, &self.jira)
    }
}

/// Provides an iterator over multiple pages of search results
#[derive(Debug)]
pub struct IssuesIter<'a> {
    jira: Jira,
    board: &'a Board,
    results: IssueResults,
    search_options: &'a SearchOptions,
}

impl<'a> IssuesIter<'a> {
    fn new(board: &'a Board, options: &'a SearchOptions, jira: &Jira) -> Result<Self> {
        let results = jira.issues().list(board, options)?;
        Ok(IssuesIter {
            board,
            jira: jira.clone(),
            results,
            search_options: options,
        })
    }

    fn more(&self) -> bool {
        (self.results.start_at + self.results.max_results) <= self.results.total
    }
}

impl<'a> Iterator for IssuesIter<'a> {
    type Item = Issue;
    fn next(&mut self) -> Option<Issue> {
        self.results.issues.pop().or_else(|| {
            if self.more() {
                match self.jira.issues().list(
                    self.board,
                    &self
                        .search_options
                        .as_builder()
                        .max_results(self.results.max_results)
                        .start_at(self.results.start_at + self.results.max_results)
                        .build(),
                ) {
                    Ok(new_results) => {
                        self.results = new_results;
                        self.results.issues.pop()
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
    }
}
