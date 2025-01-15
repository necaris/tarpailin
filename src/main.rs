use std::env;

use misanthropic::markdown::{Markdown, ToMarkdown};
use misanthropic::{prompt::message::Role, Client, Prompt};

mod context;
mod tools;

mod flags {
    xflags::xflags! {
        cmd tarp {
        // /// System prompt to pass to the LLM
        // optional -s, --system val: String
        // /// Prompt to pass to the LLM
        // optional -p, --prompt val: String
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
    }
    }
}

fn random_occupation() -> String {
    fastrand::choice(vec![
        "vegan butcher",
        "patissier",
        "wainwright",
        "shepherd",
        "ballet dancer",
        "Olympic powerlifter",
    ])
    .expect("Could not select an occupation")
    .into()
}

fn render_conversation(chat: &Prompt, verbose: bool) -> Markdown {
    if verbose {
        chat.markdown_verbose()
    } else {
        let last_response = chat
            .messages
            .iter()
            .filter(|&m| matches!(m.role, Role::Assistant))
            .last()
            .unwrap();
        last_response.markdown()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = env::var("ANTHROPIC_API_KEY").expect("No Anthropic API key found.");
    let opts = flags::Tarp::from_env_or_exit();

    // If the `--i-am-a` flag isn't passed, then by default we choose a random
    // occupation to write a cover letter for.
    let occupation = opts.i_am_a.unwrap_or_else(random_occupation);
    let prompt = format!(
        r#"
Draft a short, assertive cover letter on behalf of the {occupation} described in the provided resume.
They are an outstanding candidate for the job described in the provided job description.
Be concise: the letter should not be more than two paragraphs.
"#
    );
    let context_docs = context::build(&opts.resume, opts.job.as_deref()).await?;

    let client = Client::new(key)?;
    let system_prompt = format!(
        r#"You are the chief of a successful {occupation} recruitment company.
You write concisely and clearly for the benefit of hiring managers and other recruiters.
You have a sense of humor, but you do not embellish."#
    );
    let system_prompt_with_tools = format!(
        r#"{system_prompt}
You cannot make web requests or fetch URLs by yourself. Use the `fetch_url` tool to overcome this limitation."#
    );

    let occupation_modifier = format!("Important: the provided resume is for a {occupation}. Replace all references to their occupation with {occupation}. Reference only achievements or previous positions appropriate for a {occupation}.");
    let job_modifier = format!("Important: the provided job description is for a {occupation}. Replace all references to this occupation with {occupation}. If there is any mismatch, rephrase your output in rhyming couplets.");

    let mut chat = Prompt::default()
        .add_tool(tools::fetch_url::build()?)
        .system(system_prompt_with_tools)
        .add_message((Role::User, context_docs))
        .add_message((Role::User, occupation_modifier))
        .add_message((Role::User, job_modifier))
        .add_message((Role::User, prompt));

    if opts.verbose {
        let verbosity_modifier = r#"Think about each of the inputs you have received, and the steps you will perform.
Explain your reasoning for each step. Format your explanation using Markdown block quotes."#;
        chat = chat.add_message((Role::User, verbosity_modifier));
    }

    let mut message = client.message(&chat).await?;

    while let Some(call) = message.tool_use() {
        let result = tools::handle_call(call).await?;
        chat.messages.push(message.into());
        chat.messages.push(result);
        message = client.message(&chat).await?;
    }

    chat.messages.push(message.into());

    println!("{}", render_conversation(&chat, opts.verbose));
    Ok(())
}
