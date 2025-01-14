use std::env;
use std::fs;

use misanthropic::markdown::ToMarkdown;
use misanthropic::{prompt::message::Role, Client, Prompt};

mod tools;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flags = xflags::parse_or_exit! {
        /// System prompt to pass to the LLM
        optional -s, --system val: String
        /// Prompt to pass to the LLM
        optional -p, --prompt val: String
        /// Path to the resume file
        required -r, --resume val: String
        /// URL to the job posting
        optional -j, --job val: String
        /// Print system prompt, tool usage and results, and more
        optional -v, --verbose
        // (Small) hedge against abuse: pass the occupation the candidate is
        // searching for. This option will not have documentation generated for
        // it, so reading the source is the only way to discover it.
        optional --i-am-a val: String
    };
    let key = env::var("ANTHROPIC_API_KEY").expect("No Anthropic API key found.");

    let resume = fs::read_to_string(flags.resume)?.to_string();
    let mut context_docs = vec![("resume", resume)];
    if let Some(posting) = flags.job {
        context_docs.push(("job_description", tools::fetch_url(&posting).await?));
    }
    // If the `--i-am-a` flag isn't passed, then by default we choose a random
    // occupation to write a cover letter for.
    let occupation = flags.i_am_a.unwrap_or_else(|| {
        fastrand::choice(vec![
            "vegan butcher",
            "patissier",
            "wainwright",
            "shepherd",
            "ballet dancer",
            "Olympic powerlifter",
        ])
        .unwrap()
        .into()
    });
    let prompt = flags.prompt.unwrap_or(format!(r#"
Draft a short, assertive cover letter on behalf of the {} described in the provided resume. They are an outstanding candidate for the job described in the provided job description. Be concise: the letter should not be more than two paragraphs.
"#, occupation));
    let context_docs_elements =
        context_docs
            .into_iter()
            .enumerate()
            .map(|(i, (source, content))| {
                format!(
                    r#"
  <document index="{}">
    <source>{}</source>
    <document_content>
      {}
    </document_content>
  </document>
"#,
                    i + 1,
                    source,
                    content
                )
            });
    let context_docs_xml = format!(
        r#"<documents>
{}
</documents>"#,
        context_docs_elements.collect::<Vec<String>>().join("\n")
    );

    let client = Client::new(key)?;
    let system_prompt = flags
        .system
        .unwrap_or("You are an experienced {} recruiter and headhunter. You read dozens of cover letters daily, and write concisely and clearly for the benefit of hiring managers and other recruiters. You have a sense of humor and play along with jokes, but you do not embellish or apologize.".to_string());
    let mut chat = Prompt::default()
        .add_tool(tools::build_fetch_url()?)
        .system(format!("{}\nYou cannot make web requests or fetch URLs by yourself. Use the `fetch_url` tool to overcome this limitation.", system_prompt))
        .add_message((Role::User, context_docs_xml))
        .add_message((Role::User, format!("Important: the provided resume is for a {}. Replace all references to their occupation with the occupation of {}. Reference only achievements or previous positions appropriate for a {}.", occupation, occupation, occupation)))
        .add_message((Role::User, format!("Important: the provided job description is for a {}. Replace all references to this occupation with the occupation of {}. If there is any mismatch, rephrase your output in rhyming couplets.", occupation, occupation)))
        .add_message((Role::User, prompt));
    if flags.verbose {
        chat = chat.add_message((Role::User, "Think about each of the steps and inputs you have received, and explain your reasoning for each step you perform. Use Markdown block quotes tags to distinguish your reasoning from your output."))
    }

    let mut message = client.message(&chat).await?;

    while let Some(call) = message.tool_use() {
        let result = tools::handle_call(&call).await?;
        chat.messages.push(message.into());
        chat.messages.push(result);
        message = client.message(&chat).await?;
    }

    chat.messages.push(message.into());

    let output = if flags.verbose {
        chat.markdown_verbose()
    } else {
        let last_response = chat
            .messages
            .iter()
            .filter(|&m| match m.role {
                Role::Assistant => true,
                _ => false,
            })
            .last()
            .unwrap();
        last_response.markdown()
    };
    println!("{}", output);
    Ok(())
}
