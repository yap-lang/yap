use std::path::PathBuf;

use clap::Parser;
use lsp_server::*;
use lsp_types::*;
use tree_sitter::Node;

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
            let text = std::fs::read(path).unwrap();
            let tree = parser.parse(&text, None).unwrap();
            let expr = Expr::from((tree.root_node().child(0).unwrap(), &text[..]));
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
pub enum Expr {
    Abstraction {
        parameter: String,
        body: Box<Expr>,
    },
    Application {
        function: Box<Expr>,
        argument: Box<Expr>,
    },
    Reference {
        variable: String,
        properties: Vec<String>,
    },
    Number(f64),
    String(String),
    Record(Vec<(String, Expr)>),
}

impl From<(Node<'_>, &[u8])> for Expr {
    fn from(value: (Node<'_>, &[u8])) -> Self {
        match value.0.kind() {
            "abstraction" => Self::Abstraction {
                parameter: value
                    .0
                    .child_by_field_name("parameter")
                    .unwrap()
                    .utf8_text(value.1)
                    .unwrap()
                    .into(),
                body: Box::new((value.0.child_by_field_name("body").unwrap(), value.1).into()),
            },
            "application" => Self::Application {
                function: Box::new(
                    (value.0.child_by_field_name("function").unwrap(), value.1).into(),
                ),
                argument: Box::new(
                    (value.0.child_by_field_name("argument").unwrap(), value.1).into(),
                ),
            },
            "reference" => {
                let mut properties = Vec::new();
                {
                    let mut cursor = value.0.walk();
                    for child in value.0.children_by_field_name("property", &mut cursor) {
                        properties.push(child.utf8_text(value.1).unwrap().into());
                    }
                }

                Expr::Reference {
                    variable: value
                        .0
                        .child_by_field_name("variable")
                        .map_or(String::new(), |variable| {
                            variable.utf8_text(value.1).unwrap().into()
                        }),
                    properties,
                }
            }
            "number" => Self::Number(value.0.utf8_text(value.1).unwrap().parse().unwrap()),
            "string" => Self::String(value.0.utf8_text(value.1).unwrap().into()),
            "record" => {
                let mut cursor = value.0.walk();
                Expr::Record(
                    value
                        .0
                        .named_children(&mut cursor)
                        .map(|child| {
                            (
                                child
                                    .child_by_field_name("field")
                                    .unwrap()
                                    .utf8_text(value.1)
                                    .unwrap()
                                    .into(),
                                (child.child_by_field_name("value").unwrap(), value.1).into(),
                            )
                        })
                        .collect(),
                )
            }
            _ => todo!(),
        }
    }
}
