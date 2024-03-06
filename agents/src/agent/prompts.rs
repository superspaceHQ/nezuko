pub fn functions(add_proc: bool) -> serde_json::Value {
    let mut funcs = serde_json::json!(
        [
            {
                "name": "code",
                "description":  "Search the contents of files in a codebase semantically. Results will not necessarily match search terms exactly, but should be related.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The query with which to search. This should consist of keywords that might match something in the codebase, e.g. 'react functional components', 'contextmanager', 'bearer token'. It should NOT contain redundant words like 'usage' or 'example'."
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "path",
                "description": "Search the pathnames in a codebase. Use when you want to find a specific file or directory. Results may not be exact matches, but will be similar by some edit-distance.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The query with which path to search. This should consist of keywords that might match a path, e.g. 'server/src'."
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "none",
                "description": "Call this to answer the user. Call this only when you have enough information to answer the user's query.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {
                                "type": "integer",
                                "description": "The indices of the paths to answer with respect to. Can be empty if the answer is not related to a specific path."
                            }
                        }
                    },
                    "required": ["paths"]
                }
            },
        ]
    );

    if add_proc {
        funcs.as_array_mut().unwrap().push(
            serde_json::json!(
            {
                "name": "proc",
                "description": "Read one or more files and extract the line ranges that are relevant to the search terms",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The query with which to search the files."
                        },
                        "paths": {
                            "type": "array",
                            "items": {
                                "type": "integer",
                                "description": "The indices of the paths to search. paths.len() <= 5"
                            }
                        }
                    },
                    "required": ["query", "paths"]
                }
            }
            )
        );
    }
    funcs
}

pub fn system<'a>(paths: impl IntoIterator<Item = &'a str>) -> String {
    let mut s = "".to_string();

    let mut paths = paths.into_iter().peekable();

    if paths.peek().is_some() {
        s.push_str("## PATHS ##\nindex, path\n");
        for (i, path) in paths.enumerate() {
            s.push_str(&format!("{}, {}\n", i, path));
        }
        s.push('\n');
    }

    s.push_str(
        r#"Your job is to choose the best action. Call functions to find information that will help answer the user's query. Call functions.none when you have enough information to answer. Follow these rules at all times:

- ALWAYS call a function, DO NOT answer the question directly, even if the query is not in English
- DO NOT call a function that you've used before with the same arguments
- DO NOT assume the structure of the codebase, or the existence of files or folders
- Call functions.none with paths that you are confident will help answer the user's query
- In most cases call functions.code or functions.path functions before calling functions.none
- If the user is referring to, or asking for, information that is in your history, call functions.none
- If after attempting to gather information you are still unsure how to answer the query, call functions.none
- If the query is a greeting, or not a question or an instruction call functions.none
- When calling functions.code or functions.path, your query should consist of keywords. E.g. if the user says 'What does contextmanager do?', your query should be 'contextmanager'. If the user says 'How is contextmanager used in app', your query should be 'contextmanager app'. If the user says 'What is in the src directory', your query should be 'src'
- If functions.code or functions.path did not return any relevant information, call them again with a SIGNIFICANTLY different query. The terms in the new query should not overlap with terms in your old one
- If the output of a function is empty, try calling the function again with DIFFERENT arguments OR try calling a different function
- Only call functions.proc with path indices that are under the PATHS heading above.
- Call functions.proc with paths that might contain relevant information. Either because of the path name, or to expand on code that's already been returned by functions.code. Rank these paths based on their relevancy, and pick only the top five paths, and reject others
- DO NOT call functions.proc with more than 5 paths, it should 5 or less paths
- DO NOT call functions.proc on the same file more than once
- ALWAYS call a function. DO NOT answer the question directly"#);
    s
}

pub fn file_explanation(question: &str, path: &str, code: &str) -> String {
    format!(
        r#"Below are some lines from the file /{path}. Each line is numbered.

#####

{code}

#####

Your job is to perform the following tasks:
1. Find all the relevant line ranges of code.
2. DO NOT cite line ranges that you are not given above
3. You MUST answer with only line ranges. DO NOT answer the question

Q: find Kafka auth keys
A: [[12,15]]

Q: find where we submit payment requests
A: [[37,50]]

Q: auth code expiration
A: [[486,501],[520,560],[590,631]]

Q: library matrix multiplication
A: [[68,74],[82,85],[103,107],[187,193]]

Q: how combine result streams
A: []

Q: {question}
A: "#
    )
}

pub fn answer_article_prompt(aliases: &[usize], context: &str) -> String {
    // Return different prompts depending on whether there is one or many aliases
    let one_prompt = format!(
        r#"{context}#####

A user is looking at the code above, your job is to write an article answering their query.

Your output will be interpreted as bloop-markdown which renders with the following rules:
- Inline code must be expressed as a link to the correct line of code using the URL format: `[bar](src/foo.rs#L50)` or `[bar](src/foo.rs#L50-L54)`
- Do NOT output bare symbols. ALL symbols must include a link
  - E.g. Do not simply write `Bar`, write [`Bar`](src/bar.rs#L100-L105).
  - E.g. Do not simply write "Foos are functions that create `Foo` values out of thin air." Instead, write: "Foos are functions that create [`Foo`](src/foo.rs#L80-L120) values out of thin air."
- Only internal links to the current file work
- Basic markdown text formatting rules are allowed, and you should use titles to improve readability

Here is an example response:

A function [`openCanOfBeans`](src/beans/open.py#L7-L19) is defined. This function is used to handle the opening of beans. It includes the variable [`openCanOfBeans`](src/beans/open.py#L9) which is used to store the value of the tin opener.
"#
    );

    let many_prompt = format!(
        r#"{context}Your job is to answer a query about a codebase using the information above.

Provide only as much information and code as is necessary to answer the query, but be concise. Keep number of quoted lines to a minimum when possible. If you do not have enough information needed to answer the query, do not make up an answer.
When referring to code, you must provide an example in a code block.

Respect these rules at all times:
- Do not refer to paths by alias, expand to the full path
- Link ALL paths AND code symbols (functions, methods, fields, classes, structs, types, variables, values, definitions, directories, etc) by embedding them in a markdown link, with the URL corresponding to the full path, and the anchor following the form `LX` or `LX-LY`, where X represents the starting line number, and Y represents the ending line number, if the reference is more than one line.
  - For example, to refer to lines 50 to 78 in a sentence, respond with something like: The compiler is initialized in [`src/foo.rs`](src/foo.rs#L50-L78)
  - For example, to refer to the `new` function on a struct, respond with something like: The [`new`](src/bar.rs#L26-53) function initializes the struct
  - For example, to refer to the `foo` field on a struct and link a single line, respond with something like: The [`foo`](src/foo.rs#L138) field contains foos. Do not respond with something like [`foo`](src/foo.rs#L138-L138)
  - For example, to refer to a folder `foo`, respond with something like: The files can be found in [`foo`](path/to/foo/) folder
- Do not print out line numbers directly, only in a link
- Do not refer to more lines than necessary when creating a line range, be precise
- Do NOT output bare symbols. ALL symbols must include a link
  - E.g. Do not simply write `Bar`, write [`Bar`](src/bar.rs#L100-L105).
  - E.g. Do not simply write "Foos are functions that create `Foo` values out of thin air." Instead, write: "Foos are functions that create [`Foo`](src/foo.rs#L80-L120) values out of thin air."
- Link all fields
  - E.g. Do not simply write: "It has one main field: `foo`." Instead, write: "It has one main field: [`foo`](src/foo.rs#L193)."
- Link all symbols, even when there are multiple in one sentence
  - E.g. Do not simply write: "Bars are [`Foo`]( that return a list filled with `Bar` variants." Instead, write: "Bars are functions that return a list filled with [`Bar`](src/bar.rs#L38-L57) variants."
- Always begin your answer with an appropriate title
- Always finish your answer with a summary in a [^summary] footnote
  - If you do not have enough information needed to answer the query, do not make up an answer. Instead respond only with a [^summary] f
ootnote that asks the user for more information, e.g. `assistant: [^summary]: I'm sorry, I couldn't find what you were looking for, could you provide more information?`
- Code blocks MUST be displayed to the user using XML in the following formats:
  - Do NOT output plain markdown blocks, the user CANNOT see them
  - To create new code, you MUST mimic the following structure (example given):
###
The following demonstrates logging in JavaScript:
<GeneratedCode>
<Code>
console.log("hello world")
</Code>
<Language>JavaScript</Language>
</GeneratedCode>
###
  - To quote existing code, use the following structure (example given):
###
This is referred to in the Rust code:
<QuotedCode>
<Code>
println!("hello world!");
println!("hello world!");
</Code>
<Language>Rust</Language>
<Path>src/main.rs</Path>
<StartLine>4</StartLine>
<EndLine>5</EndLine>
</QuotedCode>
###
  - `<GeneratedCode>` and `<QuotedCode>` elements MUST contain a `<Language>` value, and `<QuotedCode>` MUST additionally contain `<Path>`, `<StartLine>`, and `<EndLine>`.
  - Note: the line range is inclusive
- When writing example code blocks, use `<GeneratedCode>`, and when quoting existing code, use `<QuotedCode>`.
- You MUST use XML code blocks instead of markdown."#
    );

    if aliases.len() == 1 {
        one_prompt
    } else {
        many_prompt
    }
}

pub fn hypothetical_document_prompt_v2(
    query: &str,
    language: &str,
    symbol_name: &str,
    symbol_type: &str,
) -> String {
    format!(
        r#"Write a code snippet in {language} language that could hypothetically be returned by a code search engine as the answer to the query: {query}

- Write the snippets in {language} language that is likely given the query
- Use a {symbol_type} named {symbol_name} while creating the snippet in language {language}
- Use the {symbol_name} more than one time in the snippet
- The snippet should be between 5 and 10 lines long
- Surround the snippet in triple backticks


For example:

Query: What's the Qdrant threshold?
language: Rust
symbol_type: function
symbol_name: SearchPoints

```rust
pub fn search_points(&self, query: &Query, filter: Option<&Filter>, top: usize) -> Result<Vec<ScoredPoint>> {{
    let mut request = SearchPoints::new(query, top);
    if let Some(filter) = filter {{
        request = request.with_filter(filter);
    }}
    let response = self.client.search_points(request).await?;
    Ok(response.points)
}}

```"#
    )
}

pub fn hypothetical_document_prompt(query: &str) -> String {
    format!(
        r#"Write a code snippet that could hypothetically be returned by a code search engine as the answer to the query: {query}

- Write the snippets in a programming or markup language that is likely given the query
- The snippet should be between 5 and 10 lines long
- Surround the snippet in triple backticks

For example:

What's the Qdrant threshold?

```rust
SearchPoints {{
    limit,
    vector: vectors.get(idx).unwrap().clone(),
    collection_name: COLLECTION_NAME.to_string(),
    offset: Some(offset),
    score_threshold: Some(0.3),
    with_payload: Some(WithPayloadSelector {{
        selector_options: Some(with_payload_selector::SelectorOptions::Enable(true)),
    }}),
```"#
    )
}

pub fn hypothetical_document_prompt_v3(
    query: &str,
    symbols: Vec<(String, String, String)>,
) -> String {
    let lang = symbols[0].clone().0;
    let symbol_type_0 = symbols[0].clone().1;
    let symbol_name_0 = symbols[0].clone().2;
    let symbol_type_1 = symbols[1].clone().1;
    let symbol_name_1 = symbols[1].clone().2;
    let symbol_type_2 = symbols[2].clone().1;
    let symbol_name_2 = symbols[2].clone().2;

    format!(
        r#"Write a code snippet in {lang} language that could hypothetically be returned by a code search engine as the answer to the query: {query}

- Write the snippets in {lang} language that is likely given the query
- Use a {symbol_name_0} named {symbol_type_0} while creating the snippet in language {lang}
- Use a {symbol_name_1} named {symbol_type_1} while creating the snippet in language {lang}
- Use a {symbol_name_2} named {symbol_type_2} while creating the snippet in language {lang}
- The snippet should be between 5 and 10 lines long
- Surround the snippet in triple backticks


For example:

Query: What's the Qdrant threshold?
language: Rust
symbol_type: function
symbol_name: search_points

symbol_type: variable 
symbol_name: request

symbol_type: module 
symbol_name: SearchPoints

```rust
use crate::SearchPoints;

pub fn search_points(&self, query: &Query, filter: Option<&Filter>, top: usize) -> Result<Vec<ScoredPoint>> {{
    let mut request = SearchPoints::new(query, top);
    if let Some(filter) = filter {{
        request = request.with_filter(filter);
    }}
    let response = self.client.search_points(request).await?;
    Ok(response.points)
}}

```"#
    )
}
pub fn try_parse_hypothetical_documents(document: &str) -> Vec<String> {
    let pattern = r"```([\s\S]*?)```";
    let re = regex::Regex::new(pattern).unwrap();

    re.captures_iter(document)
        .map(|m| m[1].trim().to_string())
        .collect()
}

pub fn question_generator_prompt(query: &str, repo_name: &str) -> String {
    let question_generator_prompt = format!(
        r#"#####

        You are a Tool  that take issue description for developer task and extract query out of to do semantic search on codebase to find relevant codes to change.
            
        Your job is to perform the following tasks:
        - Read the issue description and generate multiple query but should hold same meaning, but potentially using different keywords or phrasing.
        - If the issue description is something like \"As soon as I click on all repos, prompt guide or any button - history gets opened even if i have closed it. every action like new chat, etc gets the history opened. It should not do so\" or related  transform it to  \"["What causes the history to open when clicking on buttons?", "Why does every action trigger the history to open?", "How to prevent the history from opening with each action?"]\"
        - DO NOT CHANGE MEANING OF THE QUERY
        - RETURN ARRAY CONTAINING THREE Generated QUERIES 
        
        ----Example start----
        
        issue description- '''Current problem - In the application preview mode, the Built with ToolJet badge does not have any indication that it won't be shown once the user upgrades their plan. This is confusing to the user, that is trying to remove the badge and also does not give them an idea of the benefits of upgrading.
        '''
        
        response- ["What alterations can be made to the Built with ToolJet badge in the application preview mode to indicate that it will no longer be displayed after a user upgrades their plan?",
        "How might we adjust the Built with ToolJet badge in the preview mode to clearly demonstrate to users that it can be removed upon upgrading their plan?",
        "What modifications are required for the Built with ToolJet badge to inform users that upgrading their plan will enable them to remove the badge from the application preview mode?"
        ]

        ----Example end----
        
        GIVE ONLY ARRAY  containing  updated queries.

        DO NOT give queries in number, like 1. \"How are conversation routes implemented?\" 2. \"What is the implementation process for conversation routes?\" 3. \"Can you explain the implementation of conversation routes?\"

"#
    );
    question_generator_prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hypothetical_document() {
        let document = r#"Here is some pointless text

        ```rust
pub fn final_explanation_prompt(context: &str, query: &str, query_history: &str) -> String {
    struct Rule<'a> {
        title: &'a str,
        description: &'a str,
        note: &'a str,
        schema: &'a str,```

Here is some more pointless text

```
pub fn functions() -> serde_json::Value {
    serde_json::json!(
```"#;
        let expected = vec![
            r#"rust
pub fn final_explanation_prompt(context: &str, query: &str, query_history: &str) -> String {
    struct Rule<'a> {
        title: &'a str,
        description: &'a str,
        note: &'a str,
        schema: &'a str,"#,
            r#"pub fn functions() -> serde_json::Value {
    serde_json::json!("#,
        ];

        assert_eq!(try_parse_hypothetical_documents(document), expected);
    }
}
