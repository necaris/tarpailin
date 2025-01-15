use std::fs;

use crate::tools;

struct Document<'a> {
    source: &'a str,
    content: String,
}

impl Document<'_> {
    fn as_xml(&self, index: Option<usize>) -> String {
        let i = index.unwrap_or(0);
        format!(
            r#"  <document index="{}">
    <source>{}</source>
    <document_content>
      {}
    </document_content>
  </document>"#,
            i, self.source, self.content
        )
    }
}

pub(crate) async fn build(
    resume_path: &str,
    posting_url: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut docs = vec![load_resume(resume_path).await?];
    if let Some(url) = posting_url {
        docs.push(load_posting(url).await?);
    }
    Ok(build_xml(docs.iter()))
}

fn build_xml<'a>(documents: impl Iterator<Item = &'a Document<'a>>) -> String {
    let elements = documents
        .enumerate()
        .map(|(i, d)| d.as_xml(Some(i)))
        .collect::<Vec<String>>();

    format!("<documents>\n{}\n</documents>", elements.join("\n"))
}

async fn load_resume(path: &str) -> Result<Document, Box<dyn std::error::Error>> {
    Ok(Document {
        source: "resume",
        content: fs::read_to_string(path)?.to_string(),
    })
}

async fn load_posting(url: &str) -> Result<Document, Box<dyn std::error::Error>> {
    Ok(Document {
        source: "job description",
        content: tools::fetch_url::run(&url).await?,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_build_xml() {
        let docs = [Document {
            source: "foo",
            content: "bibbity-bobbity-boo".to_string(),
        }];
        assert_eq!(
            build_xml(docs.iter()),
            r#"<documents>
  <document index="0">
    <source>foo</source>
    <document_content>
      bibbity-bobbity-boo
    </document_content>
  </document>
</documents>"#
        );
    }
}
