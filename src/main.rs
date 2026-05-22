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
            let expr = Expression::from((&mut tree.walk(), &source[..]));
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

impl From<(&mut tree_sitter::TreeCursor<'_>, &[u8])> for Expression {
    fn from((cursor, source): (&mut tree_sitter::TreeCursor<'_>, &[u8])) -> Self {
        match cursor.node().kind() {
            "abstraction" => {
                cursor.goto_first_child(); // \
                cursor.goto_next_sibling(); // parameter
                let parameter = cursor.node().utf8_text(source).unwrap().into();
                cursor.goto_next_sibling(); // body
                let body = Self::from((&mut *cursor, source));
                cursor.goto_parent();

                Self::Abstraction {
                    parameter: parameter,
                    body: Box::new(body),
                }
            }
            "application" => {
                cursor.goto_first_child(); // function
                let function = Self::from((&mut *cursor, source));
                cursor.goto_next_sibling(); // argument
                let argument = Self::from((&mut *cursor, source));
                cursor.goto_parent();

                Self::Application {
                    function: Box::new(function),
                    argument: Box::new(argument),
                }
            }
            "reference" => {
                cursor.goto_first_child(); // . or parameter
                let parameter = if source[cursor.node().start_byte()] != b'.' {
                    Some(cursor.node().utf8_text(source).unwrap().into())
                } else {
                    None
                };

                let mut properties = Vec::new();
                loop {
                    // .
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                    cursor.goto_next_sibling(); // property
                    properties.push(cursor.node().utf8_text(source).unwrap().into());
                }
                cursor.goto_parent();

                if let Some(parameter) = parameter {
                    Self::Reference {
                        parameter,
                        properties,
                    }
                } else {
                    Self::ReferenceRoot { properties }
                }
            }
            "number" => Self::Number(cursor.node().utf8_text(source).unwrap().parse().unwrap()),
            "string" => Self::String(cursor.node().utf8_text(source).unwrap().into()),
            "record" | "root" => {
                let mut fields = Vec::new();

                cursor.goto_first_child(); // record_entry
                loop {
                    cursor.goto_first_child(); // field
                    let field = cursor.node().utf8_text(source).unwrap().into();
                    cursor.goto_next_sibling(); // :
                    cursor.goto_next_sibling(); // value
                    let value = Self::from((&mut *cursor, source));
                    cursor.goto_parent();

                    fields.push((field, value));

                    // record_entry
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();

                Self::Record(fields)
            }
            _ => unreachable!(),
        }
    }
}
