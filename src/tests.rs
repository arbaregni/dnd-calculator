use crate::*;
use crate::type_info::*;
use read_eval_print;

#[test]
fn test_add() {
    let mut env = Env::new();
    let expr = read_eval_print("10 + 2", &mut env).unwrap();
    assert_eq!(expr.try_to_num().into_owned(), 12);
}
#[test]
fn test_parens() {
    let mut env = Env::new();
    let expr = read_eval_print("(10)", &mut env).unwrap();
    assert_eq!(expr.try_to_num().into_owned(), 10);
}
#[test]
fn test_embed_parens() {
    let mut env = Env::new();
    let expr = read_eval_print("(10 + 5) * 2", &mut env).unwrap();
    assert_eq!(expr.try_to_num().into_owned(), 30);
}
#[test]
fn test_var_read() {
    let mut env = Env::new();
    env.bind_var("x".to_string(), Symbol::Num(5), Type::Num);
    let expr = read_eval_print("x", &mut env).unwrap();
    assert_eq!(expr.try_to_num().into_owned(), 5);
}
#[test]
fn test_var_assign() {
    use env::Env;
    let mut env = Env::new();
    let expr = read_eval_print("x = 2", &mut env).unwrap();
    assert!(env.lookup_var("x").is_some());
    let num: i32 = env.lookup_var("x").unwrap().0.try_to_num().into_owned();
    assert_eq!(num, 2i32)
}