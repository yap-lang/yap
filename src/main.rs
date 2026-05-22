use std::path::PathBuf;

use clap::Parser;
use lsp_server::*;
use lsp_types::*;

#[derive(clap::Parser)]
enum Command {
    Compile { path: PathBuf },
    Lsp,
}

fn main() {
    match Command::parse() {
        Command::Compile { path } => {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&tree_sitter_yap::LANGUAGE.into())
                .unwrap();

            // lexical & syntactic analysis
            let source = std::fs::read(&path).unwrap();
            let tree = parser.parse(&source, None).unwrap();
            let expr = Expression::from((tree.root_node(), &source[..]));
            println!("{:#?}", expr)

            // semantic analysis
        }
        Command::Lsp => {
            let (connection, io_threads) = Connection::stdio();
            let server_capabilities = serde_json::to_value(&ServerCapabilities {
                ..Default::default()
            })
            .unwrap();
            let _initialize_params: InitializeParams =
                serde_json::from_value(connection.initialize(server_capabilities).unwrap())
                    .unwrap();
            for message in &connection.receiver {
                match message {
                    Message::Request(_request) => {}
                    Message::Response(_response) => {}
                    Message::Notification(_notification) => {}
                }
            }
            io_threads.join().unwrap();
        }
    }
}

#[derive(Debug)]
pub enum Expression {
    Abstraction {
        parameter: String,
        body: Box<Expression>,
    },
    Application {
        function: Box<Expression>,
        argument: Box<Expression>,
    },
    Reference {
        parameter: String,
        properties: Vec<String>,
    },
    ReferenceRoot {
        properties: Vec<String>,
    },
    Number(f64),
    String(String),
    Record(Vec<(String, Expression)>),
}

impl From<(tree_sitter::Node<'_>, &[u8])> for Expression {
    fn from((node, source): (tree_sitter::Node, &[u8])) -> Self {
        match node.kind() {
            "abstraction" => Self::Abstraction {
                parameter: node
                    .child_by_field_name("parameter")
                    .unwrap()
                    .utf8_text(source)
                    .unwrap()
                    .into(),
                body: Box::new((node.child_by_field_name("body").unwrap(), source).into()),
            },
            "application" => Self::Application {
                function: Box::new((node.child_by_field_name("function").unwrap(), source).into()),
                argument: Box::new((node.child_by_field_name("argument").unwrap(), source).into()),
            },
            "reference" => {
                let properties = node
                    .children_by_field_name("property", &mut node.walk())
                    .map(|property| property.utf8_text(source).unwrap().into())
                    .collect();

                if let Some(parameter) = node.child_by_field_name("parameter") {
                    Self::Reference {
                        parameter: parameter.utf8_text(source).unwrap().into(),
                        properties,
                    }
                } else {
                    Self::ReferenceRoot { properties }
                }
            }
            "number" => Self::Number(node.utf8_text(source).unwrap().parse().unwrap()),
            "string" => Self::String(node.utf8_text(source).unwrap().into()),
            "record" | "root" => Self::Record(
                node.named_children(&mut node.walk())
                    .map(|child| {
                        (
                            child
                                .child_by_field_name("field")
                                .unwrap()
                                .utf8_text(source)
                                .unwrap()
                                .into(),
                            (child.child_by_field_name("value").unwrap(), source).into(),
                        )
                    })
                    .collect(),
            ),
            _ => unreachable!(),
        }
    }
}
