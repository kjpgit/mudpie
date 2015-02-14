use std::collections::HashSet;
use std::ascii::OwnedAsciiExt;

use super::PageFunction;
use super::WebRequest;


pub enum RoutingResult {
    FoundRule(PageFunction),
    NoPathMatch,
    NoMethodMatch(Vec<String>),
}

struct Rule {
    path: String,
    is_prefix: bool,
    methods: Vec<String>,
    page_fn: PageFunction,
}


pub struct Router {
    rules: Vec<Rule>
}


impl Router {
    pub fn new() -> Router {
        Router { rules: Vec::new() }
    }

    pub fn add_path(&mut self, methods: &str, path: &str, 
            page_fn: PageFunction, is_prefix: bool) {
        let rule = Rule { 
            path: path.to_string(), 
            is_prefix: is_prefix,
            page_fn: page_fn,
            methods: parse_methods(methods),
        };
        self.rules.push(rule);
    }

    pub fn route(&self, req: &WebRequest) -> RoutingResult {
        let mut found_path_match = false;
        let mut found_methods = HashSet::<&str>::new();

        for rule in self.rules.iter() {
            let mut matched;
            if rule.is_prefix {
                matched = req.path.starts_with(&rule.path);
            } else {
                matched = req.path == rule.path;
            }
            if matched {
                found_path_match = true;
                // Now check methods
                for method in rule.methods.iter() {
                    if *method == req.method {
                        // Found a rule match
                        return RoutingResult::FoundRule(rule.page_fn);
                    }

                    // Method doesn't match, but save it for possible error
                    found_methods.insert(&**method);
                }
            }
        }

        if found_path_match {
            // A path matched but didn't support the requested method
            // Return the available methods
            let mut methods = Vec::new();
            for method in found_methods.iter() {
                methods.push(method.to_string());
            }
            return RoutingResult::NoMethodMatch(methods);
        } else {
            return RoutingResult::NoPathMatch;
        }
    }
}


// Return: array of methods, trimmed and in lowercase
fn parse_methods(methods: &str) -> Vec<String> {
    let parts = methods.split_str(",");
    let mut ret = Vec::new();
    for p in parts {
        let method = p.trim().to_string().into_ascii_lowercase();
        ret.push(method);
    }
    return ret;
}
