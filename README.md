Experimenting with advanced type systems

Current progress: This program typechecks

```ml
type Span = (Int, Int, ())
//type Spanned 't = ('t, Span)

type TypeKind =
  | I32
  | F32
  | TypeRef of String

closed typeclass Expr<Phase: Parsed | TypeChecked> begin
  sourceSpan :: Self -> Span -> (Foo -> Bar, Int)

  [TypeChecked]
  exprType :: Self -> TypeKind


  type FuncCall = {
    ident: String,
    params: String[],
    [TypeChecked]
    retType: TypeKind
  }

  impl sourceSpan for FuncCall f = f.ident.1
  impl exprType for FuncCall f = f.retType
end



typecheck :: Expr<Parsed> -> Expr<TypeChecked>
typecheck expr =
	{ ident: "Bruh", retType: TypeKind.I32, params: [] }


main () =
	typecheck ({ ident: "foo", params: [] })

```
