pub mod syntax;
pub mod grammar;
pub mod lex;
pub mod names;
pub mod to_internal;

use lalrpop_util::ParseError;

use types;

type ParseResult<T> = Result<T, ParseError<usize, lex::Token, lex::Error>>;

pub fn ident(s: &str) -> ParseResult<syntax::Ident> {
    grammar::IdentParser::new().parse(lex::Lexer::from_str(s))
}

pub fn kind(s: &str) -> ParseResult<types::Kind> {
    grammar::KindParser::new().parse(lex::Lexer::from_str(s))
}

pub fn type_(s: &str) -> ParseResult<syntax::Type> {
    grammar::TypeParser::new().parse(lex::Lexer::from_str(s))
}

pub fn expr(s: &str) -> ParseResult<syntax::Expr> {
    grammar::ExprParser::new().parse(lex::Lexer::from_str(s))
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use super::*;
    use super::syntax::Ident;
    use expr;
    use test_utils::parse_syntax::*;

    fn name(s: &str) -> Result<String, ParseError<usize, lex::Token, lex::Error>> {
        grammar::RawNameParser::new().parse(lex::Lexer::from_str(s))
    }

    fn ws(s: &str) -> Result<(), ParseError<usize, lex::Token, lex::Error>> {
        grammar::WhitespaceParser::new().parse(lex::Lexer::from_str(s))
    }

    #[test]
    fn unquoted_name() {
        assert_eq!(name("hello"), Ok("hello".to_owned()));
        assert_eq!(name("HeLlO_wOrLd"), Ok("HeLlO_wOrLd".to_owned()));
        assert_eq!(name("_foo_bar_42_baz0"), Ok("_foo_bar_42_baz0".to_owned()));

        assert!(name("42").is_err());
        assert!(name("-hello").is_err());
        assert!(name("hello world").is_err());
    }

    #[test]
    fn quoted_name() {
        assert_eq!(name("`hello`"), Ok("hello".to_owned()));
        assert_eq!(name("`hello world`"), Ok("hello world".to_owned()));
        assert_eq!(name("`hello\\\\world`"), Ok("hello\\world".to_owned()));
        assert_eq!(name("`hello\\`world`"), Ok("hello`world".to_owned()));

        assert!(name("` ` `").is_err());
    }

    #[test]
    fn whitespace() {
        assert!(ws("").is_ok());
        assert!(ws("  \t \n    \r \x0B  \n \n \t").is_ok());

        assert!(ws("// a comment").is_ok());
        assert!(ws("   // a comment \n \t \n // another comment  \n   ").is_ok());

        assert!(ws(" - ").is_err());
        assert!(ws(" hello ").is_err());
        assert!(ws(" // a comment \n not a comment").is_err());
    }

    #[test]
    fn no_collision_ident() {
        assert_eq!(
            ident("foo"),
            Ok(Ident {
                name: "foo".to_owned(),
                collision_id: 0,
            })
        );

        assert_eq!(
            ident("`hello \\` world`"),
            Ok(Ident {
                name: "hello ` world".to_owned(),
                collision_id: 0,
            })
        );
    }

    #[test]
    fn collision_ident() {
        assert_eq!(
            ident("foo#42"),
            Ok(Ident {
                name: "foo".to_owned(),
                collision_id: 42,
            })
        );

        assert_eq!(
            ident("foo // comment 1 \n # // comment 2 \n 42"),
            Ok(Ident {
                name: "foo".to_owned(),
                collision_id: 42,
            })
        );

        assert_eq!(
            ident("`quoted ident`#005"),
            Ok(Ident {
                name: "quoted ident".to_owned(),
                collision_id: 5,
            })
        );

        assert!(ident("foo#bar").is_err());
    }

    #[test]
    fn test_kind() {
        assert_eq!(kind("*"), Ok(types::Kind::Type));
        assert_eq!(kind("Place"), Ok(types::Kind::Place));
        assert_eq!(kind("Version"), Ok(types::Kind::Version));
        assert_eq!(
            kind(
                "(((( // an embedded comment \n * // another embedded comment \n ))))",
            ),
            Ok(types::Kind::Type)
        );
        assert_eq!(
            kind("(*) -> *"),
            Ok(types::Kind::Constructor {
                params: Rc::new(vec![types::Kind::Type]),
                result: Rc::new(types::Kind::Type),
            })
        );
        assert_eq!(
            kind("(*; Place; Version) -> *"),
            Ok(types::Kind::Constructor {
                params: Rc::new(vec![
                    types::Kind::Type,
                    types::Kind::Place,
                    types::Kind::Version,
                ]),
                result: Rc::new(types::Kind::Type),
            })
        );
        assert_eq!(
            kind("(*; (*) -> *; *;) -> Place"),
            Ok(types::Kind::Constructor {
                params: Rc::new(vec![
                    types::Kind::Type,
                    types::Kind::Constructor {
                        params: Rc::new(vec![types::Kind::Type]),
                        result: Rc::new(types::Kind::Type),
                    },
                    types::Kind::Type,
                ]),
                result: Rc::new(types::Kind::Place),
            })
        );
    }

    fn ty_var(s: &str) -> syntax::Type {
        syntax::Type::Var { ident: mk_ident(s) }
    }

    #[test]
    fn test_type() {
        assert_eq!(
            type_("( // embedded whitespace \n )"),
            Ok(syntax::Type::Unit)
        );

        assert_eq!(type_("hello"), Ok(ty_var("hello")));

        assert_eq!(type_("(((((hello)))))"), Ok(ty_var("hello")));

        assert_eq!(
            type_("foo(bar)"),
            Ok(syntax::Type::App {
                constructor: Box::new(ty_var("foo")),
                param: Box::new(ty_var("bar")),
            })
        );

        assert_eq!(
            type_("foo(bar; baz)"),
            Ok(syntax::Type::App {
                constructor: Box::new(syntax::Type::App {
                    constructor: Box::new(ty_var("foo")),
                    param: Box::new(ty_var("bar")),
                }),
                param: Box::new(ty_var("baz")),
            })
        );

        assert_eq!(
            type_("foo(bar; baz;)"),
            Ok(syntax::Type::App {
                constructor: Box::new(syntax::Type::App {
                    constructor: Box::new(ty_var("foo")),
                    param: Box::new(ty_var("bar")),
                }),
                param: Box::new(ty_var("baz")),
            })
        );

        assert_eq!(
            type_("exists {t : *} t"),
            Ok(syntax::Type::Exists {
                param: syntax::TypeParam {
                    ident: mk_ident("t"),
                    kind: types::Kind::Type,
                },
                body: Box::new(ty_var("t")),
            })
        );

        assert_eq!(
            type_("foo -> bar"),
            Ok(syntax::Type::Func {
                params: Vec::new(),
                arg: Box::new(ty_var("foo")),
                ret: Box::new(ty_var("bar")),
            })
        );

        assert_eq!(
            type_("forall {t : *} t -> foo"),
            Ok(syntax::Type::Func {
                params: vec![
                    syntax::TypeParam {
                        ident: mk_ident("t"),
                        kind: types::Kind::Type,
                    },
                ],
                arg: Box::new(ty_var("t")),
                ret: Box::new(ty_var("foo")),
            })
        );

        assert_eq!(
            type_("foo, bar, baz"),
            Ok(syntax::Type::Pair {
                left: Box::new(ty_var("foo")),
                right: Box::new(syntax::Type::Pair {
                    left: Box::new(ty_var("bar")),
                    right: Box::new(ty_var("baz")),
                }),
            })
        );

        assert_eq!(
            type_("foo, bar, baz,"),
            Ok(syntax::Type::Pair {
                left: Box::new(ty_var("foo")),
                right: Box::new(syntax::Type::Pair {
                    left: Box::new(ty_var("bar")),
                    right: Box::new(ty_var("baz")),
                }),
            })
        );

        // Full example:

        assert_eq!(
            type_("exists {f : (*) -> *} (Functor(f), f(T))"),
            Ok(syntax::Type::Exists {
                param: syntax::TypeParam {
                    ident: mk_ident("f"),
                    kind: types::Kind::Constructor {
                        params: Rc::new(vec![types::Kind::Type]),
                        result: Rc::new(types::Kind::Type),
                    },
                },
                body: Box::new(syntax::Type::Pair {
                    left: Box::new(syntax::Type::App {
                        constructor: Box::new(ty_var("Functor")),
                        param: Box::new(ty_var("f")),
                    }),
                    right: Box::new(syntax::Type::App {
                        constructor: Box::new(ty_var("f")),
                        param: Box::new(ty_var("T")),
                    }),
                }),
            })
        );
    }

    fn ex_var(s: &str) -> syntax::Expr {
        syntax::Expr::Var {
            usage: expr::VarUsage::Copy,
            ident: mk_ident(s),
        }
    }

    fn ex_move_var(s: &str) -> syntax::Expr {
        syntax::Expr::Var {
            usage: expr::VarUsage::Move,
            ident: mk_ident(s),
        }
    }

    #[test]
    fn test_expr() {
        assert_eq!(
            expr("( // embedded whitespace \n )"),
            Ok(syntax::Expr::Unit),
        );

        assert_eq!(expr("hello"), Ok(ex_var("hello")));

        assert_eq!(expr("move hello"), Ok(ex_move_var("hello")));

        assert_eq!(expr("((((hello))))"), Ok(ex_var("hello")));

        assert_eq!(
            expr("hello(move world)"),
            Ok(syntax::Expr::App {
                callee: Box::new(ex_var("hello")),
                type_params: Vec::new(),
                arg: Box::new(ex_move_var("world")),
            })
        );

        assert_eq!(
            expr("hello{T}(move world)"),
            Ok(syntax::Expr::App {
                callee: Box::new(ex_var("hello")),
                type_params: vec![ty_var("T")],
                arg: Box::new(ex_move_var("world")),
            })
        );

        assert_eq!(
            expr("hello{T; U}(move world)"),
            Ok(syntax::Expr::App {
                callee: Box::new(ex_var("hello")),
                type_params: vec![ty_var("T"), ty_var("U")],
                arg: Box::new(ex_move_var("world")),
            })
        );

        assert_eq!(
            expr("hello{T; U;}(move world)"),
            Ok(syntax::Expr::App {
                callee: Box::new(ex_var("hello")),
                type_params: vec![ty_var("T"), ty_var("U")],
                arg: Box::new(ex_move_var("world")),
            })
        );

        assert_eq!(
            expr("func (x : T) -> move x"),
            Ok(syntax::Expr::Func {
                type_params: Vec::new(),
                arg_name: mk_ident("x"),
                arg_type: ty_var("T"),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("func {T : *} (x : T) -> move x"),
            Ok(syntax::Expr::Func {
                type_params: vec![
                    syntax::TypeParam {
                        ident: mk_ident("T"),
                        kind: types::Kind::Type,
                    },
                ],
                arg_name: mk_ident("x"),
                arg_type: ty_var("T"),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("func {T : *; U : *} (x : T) -> move x"),
            Ok(syntax::Expr::Func {
                type_params: vec![
                    syntax::TypeParam {
                        ident: mk_ident("T"),
                        kind: types::Kind::Type,
                    },
                    syntax::TypeParam {
                        ident: mk_ident("U"),
                        kind: types::Kind::Type,
                    },
                ],
                arg_name: mk_ident("x"),
                arg_type: ty_var("T"),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("func {T : *; U : *;} (x : T) -> move x"),
            Ok(syntax::Expr::Func {
                type_params: vec![
                    syntax::TypeParam {
                        ident: mk_ident("T"),
                        kind: types::Kind::Type,
                    },
                    syntax::TypeParam {
                        ident: mk_ident("U"),
                        kind: types::Kind::Type,
                    },
                ],
                arg_name: mk_ident("x"),
                arg_type: ty_var("T"),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("let x = move y in move x"),
            Ok(syntax::Expr::Let {
                names: vec![mk_ident("x")],
                val: Box::new(ex_move_var("y")),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("let x, y = move z in ()"),
            Ok(syntax::Expr::Let {
                names: vec![mk_ident("x"), mk_ident("y")],
                val: Box::new(ex_move_var("z")),
                body: Box::new(syntax::Expr::Unit),
            })
        );

        assert_eq!(
            expr("let x, y, = move z in ()"),
            Ok(syntax::Expr::Let {
                names: vec![mk_ident("x"), mk_ident("y")],
                val: Box::new(ex_move_var("z")),
                body: Box::new(syntax::Expr::Unit),
            })
        );

        assert_eq!(
            expr("let_exists {T} x = move y in move x"),
            Ok(syntax::Expr::LetExists {
                type_names: vec![mk_ident("T")],
                val_name: mk_ident("x"),
                val: Box::new(ex_move_var("y")),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("let_exists {T; U} x = move y in move x"),
            Ok(syntax::Expr::LetExists {
                type_names: vec![mk_ident("T"), mk_ident("U")],
                val_name: mk_ident("x"),
                val: Box::new(ex_move_var("y")),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("let_exists {T; U;} x = move y in move x"),
            Ok(syntax::Expr::LetExists {
                type_names: vec![mk_ident("T"), mk_ident("U")],
                val_name: mk_ident("x"),
                val: Box::new(ex_move_var("y")),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("make_exists {T = Foo} T of move x"),
            Ok(syntax::Expr::MakeExists {
                params: vec![(mk_ident("T"), ty_var("Foo"))],
                type_body: ty_var("T"),
                body: Box::new(ex_move_var("x")),
            })
        );

        assert_eq!(
            expr("make_exists {T = Foo; U = Bar;} T -> U of move f"),
            Ok(syntax::Expr::MakeExists {
                params: vec![
                    (mk_ident("T"), ty_var("Foo")),
                    (mk_ident("U"), ty_var("Bar")),
                ],
                type_body: syntax::Type::Func {
                    params: Vec::new(),
                    arg: Box::new(ty_var("T")),
                    ret: Box::new(ty_var("U")),
                },
                body: Box::new(ex_move_var("f")),
            })
        );

        assert_eq!(
            expr("foo, bar, baz"),
            Ok(syntax::Expr::Pair {
                left: Box::new(ex_var("foo")),
                right: Box::new(syntax::Expr::Pair {
                    left: Box::new(ex_var("bar")),
                    right: Box::new(ex_var("baz")),
                }),
            })
        );

        assert_eq!(
            expr("foo, bar, baz,"),
            Ok(syntax::Expr::Pair {
                left: Box::new(ex_var("foo")),
                right: Box::new(syntax::Expr::Pair {
                    left: Box::new(ex_var("bar")),
                    right: Box::new(ex_var("baz")),
                }),
            })
        );
    }

    // Parse an expression and convert it to an internal representation
    fn conv(
        free_vars: &[&str],
        free_types: &[&str],
        s: &str,
    ) -> Result<expr::Expr<Rc<String>>, ()> {
        let mut var_names = names::Names::new();
        for var in free_vars {
            var_names.add_name(mk_ident(var)).map_err(|_| ())?;
        }

        let mut type_names = names::Names::new();
        for ty in free_types {
            type_names.add_name(mk_ident(ty)).map_err(|_| ())?;
        }

        let result = to_internal::convert_expr(
            &mut to_internal::Context {
                var_names,
                type_names,
            },
            expr(s).map_err(|_| ())?,
        ).map_err(|_| ())?;

        assert_eq!(result.free_vars(), free_vars.len());
        assert_eq!(result.free_types(), free_types.len());

        Ok(result)
    }

    #[test]
    fn convert_expr() {
        use test_utils::expr as ex;
        use test_utils::types as ty;
        use expr::VarUsage as Usage;

        assert_eq!(conv(&[], &[], "()"), Ok(ex::unit(0, 0)));

        assert_eq!(
            conv(&[], &[], "let x = () in move x"),
            Ok(ex::let_vars_named(
                &["x"],
                ex::unit(0, 0),
                ex::var(Usage::Move, 1, 0, 0),
            ))
        );

        assert_eq!(
            conv(&[], &[], "let x, y, z = () in (x, y, z)"),
            Ok(ex::let_vars_named(
                &["x", "y", "z"],
                ex::unit(0, 0),
                ex::pair(
                    ex::var(Usage::Copy, 3, 0, 0),
                    ex::pair(
                        ex::var(Usage::Copy, 3, 0, 1),
                        ex::var(Usage::Copy, 3, 0, 2),
                    ),
                ),
            ))
        );
    }
}
