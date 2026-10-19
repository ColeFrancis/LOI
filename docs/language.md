# Language

This document serves as a reference and guide for the syntax and features of the LOI language. Throughout the document, whenever there is a blank to be filled in by some number, variable, expression, etc the "<thing>" notation is used. This tells you to mentally fill in an example of said thing.

Please note that this document only outlines the syntax of LOI. For a guide on the semantics, or meaning of the language and how to interpret and think about LOI programs, see /docs/semantics. It will be useful to read that document before this one.

# Files

All files are of the .LOI file type

# Structures

## Comments

Comments can be written in one line as follows:

```
// text
```

Or over multiple lines as follows:

```
/* text
   text
   text */
```

## Entity types

Entities, when instantiated in a network, each have a type which defines what values they can hold. Some types are built in. LOI has the following built in types:

- **Bool:** Represents a boolean value and takes either "true" or "false"
- **Int:** Represents an integer value. ex: "10", "-25". Internally stored as 64-bit signed integers
- **Float:** Represents an IEEE 64-bit floating point value. ex: "1.0", "-24.03"
- **Mod(n):** Represents an integer (mod n). Internally stored as a 64-bit signed integer
- **Impulse:** Represents a boolean that is true for a single timestep. Takes either "true" if the impulse is occuring this timestep and "false" otherwise. If set to true, will only remain true for one timestep. Represented internally as a 64-bit unsigned integer representing the simulation timestep that it will be "true"

Some types can be implicitly converted to each other. Mod can be converted to Int, Mod and Int can be converted to Float, Impulse can be converted to Bool, and Bool can be converted to Impulse. See Expressions for more details

LOI also allows you to define custom types with sets of values. Just like enums in other popular languages. The syntax is as follows:

```
ent_t <type name> = {<val name>, <val name>, ...};
```

For example:

```
ent_t COIN = {H, T};
ent_t STATE = {
    IDLE, 
    EXECUTE,
    DONE,
};
```

## Relation types

Relations, when instantiated inside networks, have functions that they evaluate to determine the output value. These functions are defined in relation types which are formatted as follows:

```
rel_t <type name> (<arg 1> : <arg 1 type>, ...) -> <return type> = <expression> ;
```

For example:

```
rel_t MULT (a: Float, b: Float) -> Float = a * b;

rel_t COUNT (coin: COIN) -> Int = cases coin {
    H : 5,
    _ : 0,
};
```

Relation types must be stateless functions, and all internal variables (see variables section) are immutable. They simply provide a mapping from a set of inputs to an output. See Networks for how to create persistant state in LOI programs.

### Expressions

Expressions are used to define the output of relation types and to define variables (see next section), inits (see Networks), etc. They come in several varieties:

**Literal**

Allow you to define a literal, unchanging value. For example:

```
rel_t Const_8 () -> Mod(3) = 2;
```

**Identifier**

Arguments or variables (see next section) can hold values like variables in any programming language. They can be used as follows:

```
rel_t RepeatInput (a: Impulse) -> Impulse = a;
```

**Unary**

Allow you to apply one of the following unary operators to an expression:

- "-": negation for Int, Float, and Mod values
- "~": boolean not for Bool and Impulse

For example:

```
rel_t NotGate (in: Bool) -> Bool = ~in;
```

**Binary**

Allow you to apply one of the following binary operations to two expressions:

- "+": addition for Int, Float, and Mod values
- "-": subtraction for Int, Float, and Mod values
- "*": multiplication for Int, Float, and Mod values 
- "/": division for Int, Float, and Mod values
- "^": exponentiation for Int, Float, and Mod values
- ">": greater than for Int, Float, and Mod values (returns Bool)
- "<": less than for Int, Float, and Mod values (returns Bool)
- ">=": greater than or equal for Int, Float, and Mod values (returns Bool)
- "<=": less than or equal for Int, Float, and Mod values (returns Bool)
- "|": or for Bool and Impulse values
- "&": and for Bool and Impulse values

**Tuple**

Tuple expressions are formatted as follows:

```
(exp1, exp2, ...)
```

They are only used for pattern matching in cases expressions (see Cases)

**Block**

Block expression allow you to embed let statements to define additional variables inside of an expression for convenience, then a final expression determines the value. Variables defined in the let statement (see variables) shadow variables from the outer scope. They are formatted as follows:

```
{stmt1; stmt2; ... expr}
```

For example:

```
{
    let a = 1;
    let b = 2;

    a + b
}

{
    let n = {
        let n = {
            1
        };

        n + 1
    };

    n + 1
}
```

The first expression would return 3, the second would also return 3.

**Cases**

Cases expressions allow for pattern matching, similar to case/match statements in other programming language. When the scrutinee expression matches one of the patterns, the expression attached to said pattern determines the value. They are formatted as follows:

```
cases <scrutinee expr> {<pattern1>: <expr1>, ..., _ : <default expr>}
```

Note that every cases expression must have a default pattern "_", whose expression is chosen if none of the other patterns match. Multiple patterns can correspond to the same arm using the boolean or operation as follows:

```
cases <scrutinee expr> {
    <pattern1> | <pattern2> | ... : <expr>,
    ...
}
```

Tuple expressions allow you to pattern with multiple variables as follows:

```
cases (<expr>, <expr>, ...) {
    (<pattern>, <pattern> ,...) : <expr>
    ...
}
```

Patterns take one of the following forms:

- Literal patterns match when the value of the scrutinee expression matches the literal
- Identifier patterns match when the value of the scrutinee expression matches the value of the variable identifier
- Comparison patterns match when the value of the scrutinee evaluates to true with the given comparison expression.

For example:

```
cases bool_var {
    true: 10,
    _ : 15,
}

cases (var1, var2) {
    (1, 1) : var1 + 1,
    (1, _) : var1 + 2,
    _      : var1 + 3,
}

cases var {
    b | c: true,
    _ : false
}

cases var {
    >= 5 : true,
    _    : false
}
```

The first expression returns 10 if bool_var is true, otherwise it returns 15. 

The second expression returns var1 + 1 if var1 and var2 are equal to 1, otherwise if var1 is 1 (no matter the value of var2), then var1 + 2 is returned, otherwise var1 + 3 is returned.

The third expression returns true if var holds the same value as b or c, otherwise it returns false.

The fourth expression returns true if var is greater than or equal to 5, otherwise it returns false.

**Sample**

Sample expressions allow you to sample a discrete random variable (continuous RVs to come in v2.~). They are formatted similar to cases expressions and the probability that each expression is returned is given. The are formatted as follows:

```
sample {<prob1> : <expr1>, ..., _ : <default expr>}
```

Each probability value must be at least 0.0 and no greater than 1.0, and the sum of all probabilites must be equal to 1.0. (The value of the default probability is assigned as 1 - prob1 - prob2 - ...). 

For example:

```
sample {
    0.5 : true,
    _   : false,
}

sample {
    a : 1,
    b : 2,
    _ : 3,
}
```

The first expression has a 0.5 probability of returning true, 0.5 of returning false.

The second expression returns 1 with probability a, 2 with probability b, otherwise 3.

**Implicit Type Conversion**

Types are implicitly converted when the the types match under the following circumstances:

- When the expression type does not match the return type, the expression type is converted to match the return type
- When two valid types are in a binary expression, the types of one is converted to match the other (This is always unambiguous. Mod->Int->Float. in the case of Bool <op> Impulse, Impulse is converted to Bool)
- When a variable of one type is matched against a variable of another type in a cases expression, the conversion happens the same as binary expressions

### Variables

LOI allows users to define variables, both globally and in relation types, for ease of programming. Variable types are inferred. They are defined as follows:

```
let <variable> = <expression>;
```

For example:

```
let a = 5;

let bernoulli = sample {p: 1, _ : 0};
```

Variables can be declared inside block expressions, or globally outside of any expressions. 

When declared outside of a rel_t, the value of a variable is determined at compile-time. When inside of a rel_t, the value is determined each time a relation of that type is evaluated.

All variables are immutable. Once defined, their value cannot be reassigned. Variables also cannot be shadowed within the same scope. For example:

```
let a = 1;

a = 4;     // Illegal

let a = 2; // Illegal

let b = {
    let a = 2; // Legal because in different scope

    a
};
```

## Networks

Networks are the core object of LOI. They allow you to organize and "connect" entities and relations in a graph-like structure. Networks are declared as follows:

```
net <name> {<net items>}
```

Several different types of net items are defined to make the creation of networks possible, and convenient.

One thing to note about networks, is that throughout a LOI program, in order to use a variable, entity type, relation type, etc it must be defined first. Entities are an exception however. One can define an entity and an input to a relation in line 10, then define it as an output of a relation in line 20, then initialize it in line 30.

### Inputs

Input net items allow you to assign an entity as an input of a network. This means that the values of said entity can be assigned or "driven" from outside of the network, like in simulation. They are declared as follows:

```
input <entity name> : <entity type> ;
```

### Outputs

Output net items allow you to assign an entity as an output of a network. This means that the values of said entity can be read from outside of the network, like in simulation. An entity can simultaneously be an input and an output. They are declared as follows:

```
output <entity name> : <entity type> ;
```

If one wants an entity to be an input and an output, it is done as follows (the order of the two lines does not matter):

```
input <entity name> : <entity type> ;
output <entity name> : <entity type> ;
```

### Inits

Inits allow you to assign an initial value to an entity, before it is ever driven by another entity. This is especially useful when entities form part of a feedback loop. See Relation instantiations for details how relations are evaluated when its input entities' values are unknown. Inits are defined as follows:

```
init <entity name> : <entity type> = <expression> ;
```

The expression value is evaluated for each instantiation of a network (see Network instantiations). This means if an init uses a sample expression, the assigned value could be different for different instantiations of that network.

For example, one would define an entity as an output and give it an initial value as follows:

```
output ent_a : Float;

init ent_a : Float = 1.0;
```

### Relation instantiations

Relation instantiations allow you to place relations in a network and define their input and output entities. They are defined as follows:

```
<rel_t name> (<input entity 1>, <input entity 2>, ...) := <output entity> ;
```

For example, to define a relation and place it inside a network, you would do as follows:


```
rel_t ADD (a: Int, b: Int) -> Int = a + b;
rel_t MULT (a: Int, b: Int) -> Int = a * b;

net MULT_ADD {
    input a: Int;
    input b: Int;
    input c: Int;
    input d: Int;

    output q: Int;

    MULT(a, b) := inter_1;

    MULT(c, d) := inter_2;

    ADD(inter_1, inter_2) := q;
}
```

In this example, a, b, c, d, inter_1, inter_2, and q are entities, all of type Int. There are three relations, two of type MULT, one of type ADD.

Note that in a relation instantiation, all input arguments of the relation type must be provided.

Also note, that an entity cannot be driven more than once. Therefore, an entity cannot simultaneously be an input and the output of a relation

### Network instantiations

When networks have repeated structures, it is often useful, if not critical to take advantage of that structure. This is achieved in LOI through nesting networks recursively inside one another. 

We will define a port of a network as an entity that is either an input or an output or said network.

Network instantiations allow you to nest one network in another. This is done as follows:

```
<instantiated net> {<port 1> := <outer entity 1>, ...};
```

For example:

```
net INNER {
    input a: Bool;
    input b: Bool;

    output q: Bool;

    AND(a, b) := inter;
    NOT(inter) := q;
}

net OUTER {
    input a: Bool;
    input b: Bool;
    input c: Bool;

    output out: Bool;

    INNER {
        a := a,
        b := b,
        q := inter,
    };
    INNER {
        a := inter,
        b := c,
        q := out,
    };
}
```

In this example, the OUTER network instantiates the INNER network twice. For the first, OUTER assigns its entities a and b to the input ports a and b of INNER respectively, and its entity inter to the q port of INNER. For the second, OUTER assigns its entities inter and c to the input ports a and b of INNER respectively, and its entity out to the q port of INNER. The following code would produce the same underlying network:

```
net OUTER {
    input a: Bool;
    input b: Bool;
    input c: Bool;

    output out: Bool;

    AND(a, b) := inter_1;
    NOT(inter_1) := inter_2;

    AND(inter_2, c) := inter_3;
    NOT(inter_3) := out;
}
```

Note that not all ports of an instantiated network need to be assigned to an entity of the outer network.

Also note, that the ports in a network instantiation don't have to be assigned in order. The following are equally valid and equivalent:

```
A {
    a := foo,
    b := bar,
};

A {
    b := bar,
    a := foo,
};
```

Also note again that an entity cannot be driven more than once. Therefore, an entity cannot simultaneously be and input and connected to an output port of an instantiated network.

Also note that init values are evaluated for each separate instantiation of a net. Therefore, in the following example, entity_1 and entity_2 could have different values:

```
net RAND {
    output a: Bool;

    init a: Bool = sample {
        0.5 : true,
        _   : false,
    };
}

net EXAMPLE {
    RAND {
        a := entity_1
    };

    RAND {
        a := entity_2
    };
}
```
