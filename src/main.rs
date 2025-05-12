use oxc::ast::ast::{Statement, TSSignature, TSType, TSTypeParameter};
use std::{fs, path::Path};
use oxc::allocator::Allocator;
use oxc::parser::{ParseOptions, Parser};
use oxc::span::SourceType;
use oxc::semantic::ScopeFlags;

use oxc::ast::ast::*;

use oxc::ast_visit::{Visit, walk};

struct PrintVisitor;

impl<'a> Visit<'a> for PrintVisitor {
    fn visit_ts_interface_declaration(&mut self, it: &TSInterfaceDeclaration<'a>) {
        println!("Interface: {}", it.id.name);
        walk::walk_ts_interface_body(self, &it.body);
    }
    fn visit_identifier_name(&mut self, identifier: &IdentifierName<'a>) {
        println!("\tIdentifier: {}", identifier.name);
    }
    fn visit_ts_type(&mut self, ts_type: &TSType<'a>) {
        println!("\t\tType: {}", process_type(ts_type).unwrap());
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
          reference.type_name
            .get_identifier_reference()
            .name
            .as_str(), 
        _ => "Err" 
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
                result.push_str(format!("\n\tType: {}", process_type(&type_annotation.type_annotation).unwrap()).as_str());
            }
            // process_type(prop_sig.type_annotation.unwrap().type_annotation);
            // result.push_str(format!("\n\tType: {}", process_type_annotation(prop_sig.type_annotation)).as_str());
            Ok(result)
        }
        _ => Err("Error processing signature".to_string())
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
        _ => Err("Error processing statement".to_string())
    }
}
fn main() -> Result<(), String> {
    let source_text = fs::read_to_string("test.d.ts").map_err(|_| "Missing 'test.ts'".to_string())?;
    let source_type = SourceType::from_path(Path::new("test.d.ts")).unwrap();

    // let source_text = fs::read_to_string("../node_modules/@types/web/index.d.ts").map_err(|_| "Missing 'test.ts'".to_string())?;
    // let source_type = SourceType::from_path(Path::new("../node_modules/@types/web/index.d.ts")).unwrap();
    let allocator = Allocator::default();
    
    let ret = Parser::new(&allocator, &source_text, source_type)
        .with_options(ParseOptions { parse_regular_expression: true, ..ParseOptions::default() })
        .parse();

    let mut visitor = PrintVisitor;
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
