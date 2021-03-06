use std::{collections::HashMap, fmt::Debug, rc::Rc};

use crate::{ast::typed::TypedExpr, ast::typed::*, typecheck::TypeChecked};

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Tuple(Vec<Value>),
    Function(Rc<String>, Vec<(String, Value)>, *const TypedExpr),
    String(Rc<String>),
    Integer(i64),
    Variant(TypeHandle, usize, Rc<Value>),
    VariantConstructorFn(TypeHandle, usize),
    BuiltInFn(BuiltInFn),
}

#[derive(Debug)]
struct Interpreter {
    stack: Vec<Value>,
    bindings: HashMap<String, Value>,
    program: TypeChecked,
}

impl Interpreter {
    pub fn push_val(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn pop_val(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    pub fn call_fn(&mut self, f: &str) {
        let (e, _t) = {
            let env = self.program.environment.borrow();

            env.root_scope
                .bindings
                .get(f)
                .expect(&format!("function not found: {}", f))
                .clone()
        };

        if let ExprT::Lambda(_p, body) = e.clone() {
            self.eval_expr(&body)
        } else {
            panic!("Tried to call non function value {:?}", e);
        }
    }

    pub fn call_builtin(&mut self, builtin: BuiltInFn, arg: Value) {
        match builtin {
            BuiltInFn::FileRead => {
                if let Value::String(s) = arg {
                    let buf = std::fs::read_to_string(s.as_str()).unwrap();
                    self.push_val(Value::String(Rc::new(buf)));
                } else {
                    panic!()
                }
            }
            BuiltInFn::Print => {
                if let Value::String(s) = arg {
                    print!("{}", s);
                    self.push_val(Value::Unit);
                } else {
                    panic!();
                }
            }
            BuiltInFn::Printi => {
                if let Value::Integer(i) = arg {
                    print!("{}", i);
                    self.push_val(Value::Unit);
                } else {
                    panic!()
                }
            }
            BuiltInFn::StringParseInt => {
                if let Value::String(s) = arg {
                    self.push_val(Value::Integer(s.parse::<i64>().unwrap()));
                } else {
                    panic!()
                }
            }
            BuiltInFn::StringGetFirst => {
                if let Value::String(s) = arg {
                    self.push_val(Value::Tuple(vec![
                        Value::String(Rc::new(s[0..1].to_string())),
                        Value::String(Rc::new(s[1..].to_string())),
                    ]));
                } else {
                    panic!();
                }
            }
            BuiltInFn::StringSplit => {
                if let Value::Tuple(args) = arg {
                    assert!(args.len() == 2);
                    match (&args[0], &args[1]) {
                        (Value::String(input), Value::String(seperator)) => {
                            if let Some(sep_i) = input.find(seperator.as_str()) {
                                let (up, to) = input.split_at(sep_i);
                                self.push_val(Value::Tuple(vec![
                                    Value::String(Rc::new(up.to_string())),
                                    Value::String(Rc::new(to[seperator.len()..].to_owned())),
                                ]));
                            } else {
                                self.push_val(Value::Tuple(vec![
                                    Value::String(input.clone()),
                                    Value::String(Rc::new(String::new())),
                                ]));
                            }
                        }
                        _ => panic!(),
                    }
                } else {
                    panic!()
                }
            }
            _ => {
                dbg!(builtin);

                unimplemented!()
            }
        }
    }

    pub fn eval_expr(&mut self, (expr, _et): &TypedExpr) {
        match expr {
            ExprT::Tuple(exprs) => {
                let mut vals = Vec::new();
                for e in exprs {
                    self.eval_expr(e);
                    vals.push(self.pop_val().unwrap());
                }
                self.push_val(Value::Tuple(vals));
            }
            ExprT::LetBinding(binding, rhs, body) => {
                self.eval_expr(rhs);
                let rv = self.pop_val().unwrap();
                self.bindings.insert(binding.clone(), rv);

                self.eval_expr(body);
                self.bindings.remove(binding);
            }
            ExprT::MatchSum(matchee, arms) => {
                self.eval_expr(matchee);

                if let Some(Value::Variant(th, vi, val)) = self.pop_val() {
                    for (arm_i, binding, body) in arms {
                        if *arm_i == vi {
                            binding.iter().for_each(|binding| {
                                self.bindings.insert(binding.clone(), (*val).clone());
                            });

                            self.eval_expr(body);

                            binding.iter().for_each(|binding| {
                                self.bindings.remove(binding);
                            });

                            return;
                        }
                    }

                    panic!("{:?}, {:?}", arms, vi)
                } else {
                    panic!()
                }
            }
            ExprT::Application(lhs, rhs) => {
                self.eval_expr(lhs);

                for expr in rhs {
                    let top = self.pop_val();
                    if let Some(Value::Function(p, curried, body)) = top {
                        // scoping
                        self.eval_expr(expr);
                        let rv = self.pop_val().unwrap();
                        let bindings_tmp = self.bindings.clone();
                        self.bindings.clear();

                        for (i, e) in curried.clone() {
                            self.bindings.insert(i, e);
                        }
                        self.bindings.insert((*p).clone(), rv);

                        self.eval_expr(unsafe { &*body });

                        self.bindings = bindings_tmp;
                    } else if let Some(Value::VariantConstructorFn(th, vi)) = top {
                        self.eval_expr(expr);
                        let rv = self.pop_val().unwrap();
                        self.push_val(Value::Variant(th.clone(), vi, Rc::new(rv)));
                    } else if let Some(Value::BuiltInFn(f)) = top {
                        self.eval_expr(expr);
                        let argv = self.pop_val().unwrap();
                        self.call_builtin(f, argv);
                    } else {
                        dbg!(lhs, top, &self.stack, &self.bindings);
                        panic!("Not good")
                    }
                }
            }
            ExprT::Lambda(p, body) => {
                self.push_val(Value::Function(
                    Rc::new(p.clone()),
                    self.bindings.clone().into_iter().collect(),
                    body.as_ref() as *const TypedExpr,
                ));
            }
            ExprT::BooleanLiteral(b) => self.push_val(Value::Integer(*b as i64)),
            ExprT::Conditional(cond, cons, alt) => {
                self.eval_expr(cond);

                if let Value::Integer(0) = self.pop_val().unwrap() {
                    self.eval_expr(alt);
                } else {
                    self.eval_expr(cons);
                }
            }
            ExprT::Symbol(s) => {
                let val = {
                    let env = self.program.environment.borrow();

                    let b = env.root_scope.bindings.get(s);

                    if let Some(b) = b {
                        if let (ExprT::Lambda(p, body), _) = b {
                            Value::Function(
                                Rc::new(p.clone()),
                                vec![],
                                body.as_ref() as *const TypedExpr,
                            )
                        } else if let (ExprT::BuiltInFn(f), _) = b {
                            Value::BuiltInFn(*f)
                        } else {
                            panic!()
                        }
                    } else if let Some(b) = self.bindings.get(s).cloned() {
                        b
                    } else {
                        panic!("{:?}", s)
                    }
                };
                self.push_val(val);
            }
            ExprT::Record(fields) => {
                let mut r = Vec::new();
                for f in fields {
                    self.eval_expr(f);
                    r.push(self.pop_val().unwrap());
                }
                self.push_val(Value::Tuple(r));
            }
            ExprT::BinaryOp(op, lhs, rhs) => {
                self.eval_expr(lhs);
                self.eval_expr(rhs);

                use crate::ast::untyped::Operator;

                match (self.pop_val().unwrap(), self.pop_val().unwrap()) {
                    (Value::Integer(r), Value::Integer(l)) => {
                        let r = match op {
                            Operator::BinOpAdd => l + r,
                            Operator::BinOpSub => l - r,
                            Operator::BinOpMul => l * r,
                            Operator::BinOpDiv => l / r,
                            Operator::BinOpLess => (l < r) as i64,
                            Operator::BinOpLessEq => (l <= r) as i64,
                            Operator::BinOpGreater => (l > r) as i64,
                            Operator::BinOpGreaterEq => (l >= r) as i64,
                            Operator::BinOpEquals => (l == r) as i64,
                            Operator::BinOpAnd => (l & r) as i64,
                            Operator::BinOpOr => (l | r) as i64,
                            Operator::BinOpMod => (l % r),
                            _ => panic!(),
                        };

                        self.push_val(Value::Integer(r));
                    }
                    (Value::String(r), Value::String(l)) => match op {
                        Operator::BinOpEquals => {
                            self.push_val(Value::Integer((l == r) as i64));
                        }
                        _ => panic!(),
                    },
                    _ => panic!(),
                }
            }
            ExprT::StringLiteral(s) => {
                self.push_val(Value::String(Rc::new(s.clone())));
            }
            ExprT::IntegerLiteral(i) => self.push_val(Value::Integer(*i)),
            ExprT::VariantConstructor(th, vi) => {
                let t = self.program.environment.borrow().types[th.index].clone();
                if let TypeDefinition::Sum { variants, .. } = t {
                    let (_n, vt) = &variants[*vi];
                    {
                        self.push_val(Value::VariantConstructorFn(th.clone(), *vi));
                    }
                } else {
                    panic!()
                }
            }
            ExprT::BuiltInFn(f) => {
                self.push_val(Value::BuiltInFn(f.clone()));
            }
            ExprT::FieldAccess(lhs, i) => {
                self.eval_expr(lhs);

                if let Some(Value::Tuple(values)) = self.pop_val() {
                    self.push_val(values[*i].clone())
                } else {
                    panic!()
                }
            }
            ExprT::Unit => self.push_val(Value::Unit),
            _ => {
                dbg!(expr);
                unimplemented!()
            }
        }
    }
}

pub fn interpret(program: TypeChecked) {
    let mut interpreter = Interpreter {
        bindings: HashMap::new(),
        stack: Vec::new(),
        program,
    };

    interpreter.call_fn("main");

    dbg!(interpreter.pop_val());
}
