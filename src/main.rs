use oxc::allocator::Allocator;
use oxc::ast::ast::{Statement, TSSignature, TSType};
use oxc::parser::{ParseOptions, Parser};
use oxc::span::SourceType;
use std::fmt::format;
use std::{fs, path::Path};

use oxc::ast::ast::*;

use oxc::ast_visit::{Visit, walk};

#[derive(Debug, Default)]
struct PrintVisitor {
    tab: usize,
}
impl PrintVisitor {
    fn println_str(&mut self, value: &str) {
        self.println(String::from(value));
    }
    fn println(&mut self, value: String) {
        println!("{:indent$}{}", " ", value, indent = self.tab);
    }
    fn incr(&mut self) {
        self.tab += 4
    }
    fn decr(&mut self) {
        self.tab -= 4
    }
}

impl<'a> Visit<'a> for PrintVisitor {
    fn visit_ts_interface_declaration(&mut self, it: &TSInterfaceDeclaration<'a>) {
        println!("<interface>");
        self.incr();
        self.println(format!("<name>{}</name>", it.id.name));

        match &it.type_parameters {
            Some(param) => walk::walk_ts_type_parameters(self, &param.params),
            None => (),
        }

        //TODO: no idea why visit_ts_interface_heritages isn't working
        for heritage in it.extends.iter() {
            print!(
                "Extends: {}",
                //yeesh
                heritage.expression.get_identifier_reference().unwrap().name
            );
            match &heritage.type_arguments {
                Some(args) => {
                    let mut result_list: Vec<String> = Vec::new();
                    for arg in args.params.iter() {
                        result_list.push(String::from(arg.get_identifier_reference().unwrap().name))
                    }
                    println!("<{}>", result_list.join(", "))
                }
                None => println!("e"),
            }
        }
        //TODO: remove the above and replace with this walk
        self.visit_ts_interface_heritages(&it.extends);
        self.visit_ts_interface_body(&it.body);
        self.decr();
        self.println_str("</interface>");
    }

    fn visit_ts_type_parameter(&mut self, it: &TSTypeParameter<'a>) {
        self.println(format!("<parameter>{}</parameter>", it.name.name));
    }
    // fn visit_ts_type_parameter_instantiation(&mut self, it: &TSTypeParameterInstantiation<'a>) {
    //     println!("\tParameter Instantiation: {}", it.)
    // }
    fn visit_ts_interface_heritages(
        &mut self,
        it: &oxc::allocator::Vec<'a, TSInterfaceHeritage<'a>>,
    ) {
        println!("<extends>");
        walk::walk_ts_interface_heritages(self, it);
        println!("</extends>");
    }
    fn visit_ts_interface_heritage(&mut self, it: &TSInterfaceHeritage<'a>) {
        println!(
            "Something: {}",
            it.expression.get_identifier_reference().unwrap().name
        );
    }
    fn visit_ts_interface_body(&mut self, it: &TSInterfaceBody<'a>) {
        self.println_str("<body>");
        self.incr();
        walk::walk_ts_interface_body(self, it);
        self.decr();
        self.println_str("</body>");
    }
    fn visit_ts_property_signature(&mut self, it: &TSPropertySignature<'a>) {
        self.println_str("<property>");
        self.incr();
        walk::walk_ts_property_signature(self, it);
        self.decr();
        self.println_str("</property>");
    }
    fn visit_identifier_name(&mut self, identifier: &IdentifierName<'a>) {
        self.println(format!("<name>{}</name>", identifier.name));
    }
    fn visit_ts_type(&mut self, ts_type: &TSType<'a>) {
        self.println(format!("<type>{}</type>", process_type(ts_type).unwrap()));
    }
}

fn process_type(ts_type: &TSType) -> Result<String, String> {
    let result = match ts_type {
        TSType::TSAnyKeyword(_) => "any",
        TSType::TSBigIntKeyword(_) => "bigint",
        TSType::TSBooleanKeyword(_) => "boolean",
        TSType::TSNeverKeyword(_) => "never",
        TSType::TSNullKeyword(_) => "null",
        TSType::TSNumberKeyword(_) => "number",
        TSType::TSObjectKeyword(_) => "object",
        TSType::TSStringKeyword(_) => "string",
        TSType::TSUndefinedKeyword(_) => "undefined",
        TSType::TSUnknownKeyword(_) => "unknown",
        TSType::TSVoidKeyword(_) => "void",
        TSType::TSTypeReference(reference) =>
        //TODO: handle type arguments
        {
            reference.type_name.get_identifier_reference().name.as_str()
        }
        _ => "Err",
    };
    if result == "Err" {
        Err(result.to_string())
    } else {
        Ok(result.to_string())
    }
}

// fn process_type_parameters(param: &TSTypeParameter) -> Result<String, String> {
//
// }

fn process_signature(signature: &TSSignature) -> Result<String, String> {
    match signature {
        TSSignature::TSPropertySignature(prop_sig) => {
            let mut result = String::new();
            // result.push_str(format!("{:?}", prop_sig).as_str());
            result.push_str(format!("\nProperty: {}", prop_sig.key.name().unwrap()).as_str());
            if let Some(type_annotation) = prop_sig.type_annotation.as_ref() {
                result.push_str(
                    format!(
                        "\n\tType: {}",
                        process_type(&type_annotation.type_annotation).unwrap()
                    )
                    .as_str(),
                );
            }
            // process_type(prop_sig.type_annotation.unwrap().type_annotation);
            // result.push_str(format!("\n\tType: {}", process_type_annotation(prop_sig.type_annotation)).as_str());
            Ok(result)
        }
        _ => Err("Error processing signature".to_string()),
    }
}

fn process_statement(stmt: &Statement) -> Result<String, String> {
    match stmt {
        Statement::TSInterfaceDeclaration(interface) => {
            let mut result = String::new();
            // result.push_str(format!("{:?}", interface).as_str());
            result.push_str(format!("\nName: {}", interface.id.name).as_str());
            // result.push_str(format!("\nName: {}", process_body(&interface.body.body).unwrap()).as_str());
            for stmt in &interface.body.body {
                result.push_str(process_signature(stmt).unwrap().as_str());
            }
            Ok(result)
        }
        _ => Err("Error processing statement".to_string()),
    }
}
fn main() -> Result<(), String> {
    let source_text =
        fs::read_to_string("test.d.ts").map_err(|_| "Missing 'test.ts'".to_string())?;
    let source_type = SourceType::from_path(Path::new("test.d.ts")).unwrap();

    // let source_text = fs::read_to_string("../node_modules/@types/web/index.d.ts").map_err(|_| "Missing 'test.ts'".to_string())?;
    // let source_type = SourceType::from_path(Path::new("../node_modules/@types/web/index.d.ts")).unwrap();
    let allocator = Allocator::default();

    let ret = Parser::new(&allocator, &source_text, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();

    let mut visitor = PrintVisitor::default();
    visitor.visit_program(&ret.program);
    // ret.program(&mut visitor); //
    // if ret.errors.is_empty() {
    //     println!("Parsed Successfully.");
    //     // println!("AST:\n{:#?}", ret.program); // Pr
    //     for stmt in &ret.program.body {
    //         println!("Statement: {}\n\n\n", process_statement(stmt).unwrap());
    //     }
    // } else {
    //     for error in ret.errors {
    //         println!("{:?}", error);
    //     }
    //     println!("Parsed with Errors.");
    // }

    Ok(())
}
