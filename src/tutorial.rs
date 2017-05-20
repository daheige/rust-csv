/*!
A tutorial for handling CSV data in Rust.

This tutorial is targeted at beginner Rust programmers, but experienced Rust
programmers may find parts of this tutorial useful as well. This tutorial will
cover basic CSV reading and writing, automatic (de)serialization with Serde,
CSV transformations and performance.

For an introduction to Rust, please see the
[official book](https://doc.rust-lang.org/beta/book/second-edition/).
If you haven't written any Rust code yet but have written code in another
language, then this tutorial might be accessible to you without needing to read
the book first.

# Table of contents

1. [Setup](#setup)
1. [Basic error handling](#basic-error-handling)
    * [Switch to recoverable errors](#switch-to-recoverable-errors)
1. [Reading CSV](#reading-csv)
    * [Reading headers](#reading-headers)
    * [Delimiters, quotes and variable length records](#delimiters-quotes-and-variable-length-records)
    * [Reading with Serde](#reading-with-serde)
    * [Handling invalid data with Serde](#handling-invalid-data-with-serde)
1. [Writing CSV](#writing-csv)
    * [Writing tab separated values](#writing-tab-separated-values)
    * [Writing with Serde](#writing-with-serde)
1. [Pipelining](#pipelining)
    * [Filter by search](#filter-by-search)
    * [Filter by population count](#filter-by-population-count)
1. [Project: concatenate CSV data](#project-concatenate-csv-data)
1. [Performance](#performance)
    * [Amortizing allocations](#amortizing-allocations)
    * [Serde and zero allocation](#serde-and-zero-allocation)
    * [CSV parsing without the standard library](#csv-parsing-without-the-standard-library)
1. [Closing thoughts](#closing-thoughts)

# Setup

In this section, we'll get you setup with a simple program that reads CSV data
and prints a "debug" version of each record. This assumes that you have the
[Rust toolchain installed](https://www.rust-lang.org/install.html),
which includes both Rust and Cargo.

We'll start by creating a new Cargo project:

```text
$ cargo new --bin csvtutor
$ cd csvtutor
```

Once inside `csvtutor`, open `Cargo.toml` in your favorite text editor and add
`csv = "1.0.0-beta.1"` to your `[dependencies]` section. At this point, your
`Cargo.toml` should look something like this:

```text
[package]
name = "csvtutor"
version = "0.1.0"
authors = ["Your Name"]

[dependencies]
csv = "1.0.0-beta.1"
```

Next, let's build your project. Since you added the `csv` crate as a
dependency, Cargo will automatically download it and compile it for you. To
build your project, use Cargo:

```text
$ cargo build
```

This will produce a new binary, `csvtutor`, in your `target/debug` directory.
It won't do much at this point, but you can run it:

```text
$ ./target/debug/csvtutor
Hello, world!
```

Let's make our program do something useful. Our program will read CSV data on
stdin and print debug output for each record on stdout. To write this program,
open `src/main.rs` in your favorite text editor and replace its contents with
this:

```no_run
//tutorial-setup-01.rs
// This makes the csv crate accessible to your program.
extern crate csv;

// Import the standard library's I/O module so we can read from stdin.
use std::io;

// The `main` function is where your program starts executing.
fn main() {
    // Create a CSV parser that reads data from stdin.
    let mut rdr = csv::Reader::from_reader(io::stdin());
    // Loop over each record.
    for result in rdr.records() {
        // An error may occur, so abort the program in an unfriendly way.
        // We will make this more friendly later!
        let record = result.expect("a CSV record");
        // Print a debug version of the record.
        println!("{:?}", record);
    }
}
```

Don't worry too much about what this code means; we'll dissect it in the next
section. For now, try rebuilding your project:

```text
$ cargo build
```

Assuming that succeeds, let's try running our program. But first, we will need
some CSV data to play with! For that, we will use a random selection of 100
US cities, along with their population size and geographical coordinates. (We
will use this same CSV data throughout the entire tutorial.) To get the data,
download it from github:

```text
$ curl -LO 'https://raw.githubusercontent.com/BurntSushi/rust-csv/rewrite/examples/data/uspop.csv'
```

And now finally, run your program on `uspop.csv`:

```text
$ ./target/debug/csvtutor < uspop.csv
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
# ... and much more
```

# Basic error handling

Since reading CSV data can result in errors, error handling is pervasive
throughout the examples in this tutorial. Therefore, we're going to spend a
little bit of time going over basic error handling, and in particular, fix
our previous example to show errors in a more friendly way. **If you're already
comfortable with things like `Result` and `try!`/`?` in Rust, then you can
safely skip this section.**

Note that
[The Rust Programming Language Book](https://doc.rust-lang.org/beta/book/second-edition/)
contains an
[introduction to general error handling](https://doc.rust-lang.org/beta/book/second-edition/ch09-00-error-handling.html).
For a deeper dive, see
[my blog post on error handling in Rust](http://blog.burntsushi.net/rust-error-handling/).
The blog post is especially important if you plan on building Rust libraries.

With that out of the way, error handling in Rust comes in two different forms:
unrecoverable errors and recoverable errors.

Unrecoverable errors generally correspond to things like bugs in your program,
which might occur when an invariant or contract is broken. At that point, the
state of your program is unpredictable, and there's typically little recourse
other than *panicking*. In Rust, a panic is similar to simply aborting your
program, but it will unwind the stack and clean up resources before your
program exits.

On the other hand, recoverable errors generally correspond to predictable
errors. A non-existent file or invalid CSV data are examples of recoverable
errors. In Rust, recoverable errors are handled via `Result`. A `Result`
represents the state of a computation that has either succeeded or failed.
It is defined like so:

```
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

That is, a `Result` either contains a value of type `T` when the computation
succeeds, or it contains a value of type `E` when the computation fails.

The relationship between unrecoverable errors and recoverable errors is
important. In particular, it is **strongly discouraged** to treat recoverable
errors as if they were unrecoverable. For example, panicking when a file could
not be found, or if some CSV data is invalid, is considered bad practice.
Instead, predictable errors should be handled using Rust's `Result` type.

With our new found knowledge, let's re-examine our previous example and dissect
its error handling.

```no_run
//tutorial-error-01.rs
extern crate csv;

use std::io;

fn main() {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        let record = result.expect("a CSV record");
        println!("{:?}", record);
    }
}
```

There are two places where an error can occur in this program. The first is
if there was a problem reading a record from stdin. The second is if there is
a problem writing to stdout. In general, we will ignore the latter problem in
this tutorial, although robust command line applications should probably try
to handle it (e.g., when a broken pipe occurs). The former however is worth
looking into in more detail. For example, if a user of this program provides
invalid CSV data, then the program will panic:

```text
$ cat invalid
header1,header2
foo,bar
quux,baz,foobar
$ ./target/debug/csvtutor < invalid
StringRecord { position: Some(Position { byte: 16, line: 2, record: 1 }), fields: ["foo", "bar"] }
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: UnequalLengths { pos: Some(Position { byte: 24, line: 3, record: 2 }), expected_len: 2, len: 3 }', /checkout/src/libcore/result.rs:859
note: Run with `RUST_BACKTRACE=1` for a backtrace.
```

What happened here? First and foremost, we should talk about why the CSV data
is invalid. The CSV data consists of three records: a header and two data
records. The header and first data record have two fields, but the second
data record has three fields. By default, the csv crate will treat inconsistent
record lengths as an error.
(This behavior can be toggled using the
[`ReaderBuilder::flexible`](../struct.ReaderBuilder.html#method.flexible)
config knob.) This explains why the first data record is printed in this
example, since it has the same number of fields as the header record. That is,
we don't actually hit an error until we parse the second data record.

(Note that the CSV reader automatically interprets the first record as a
header. This can be toggled with the
[`ReaderBuilder::has_headers`](../struct.ReaderBuilder.html#method.has_headers)
config knob.)

So what actually causes the panic to happen in our program? That would be the
first line in our loop:

```ignore
for result in rdr.records() {
    let record = result.expect("a CSV record"); // this panics
    println!("{:?}", record);
}
```

The key thing to understand here is that `rdr.records()` returns an iterator
that yields `Result` values. That is, instead of yielding records, it yields
a `Result` that contains either a record or an error. The `expect` method,
which is defined on `Result`, *unwraps* the success value inside the `Result`.
Since the `Result` might contain an error instead, `expect` will *panic* when
it does contain an error.

It might help to look at the implementation of `expect`:

```ignore
use std::fmt;

// This says, "for all types T and E, where E can be turned into a human
// readable debug message, define the `expect` method."
impl<T, E: fmt::Debug> Result<T, E> {
    fn expect(self, msg: &str) -> T {
        match self {
            Ok(t) => t,
            Err(e) => panic!("{}: {:?}", msg, e),
        }
    }
}
```

Since this causes a panic if the CSV data is invalid, and invalid CSV data is
a perfectly predictable error, we've turned what should be a *recoverable*
error into an *unrecoverable* error. We did this because it is expedient to
use unrecoverable errors. Since this is bad practice, we will endeavor to avoid
unrecoverable errors throughout the rest of the tutorial.

## Switch to recoverable errors

We'll convert our unrecoverable error to a recoverable error in 3 steps. First,
let's get rid of the panic and print an error message manually:

```no_run
//tutorial-error-02.rs
extern crate csv;

use std::io;
use std::process;

fn main() {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // Examine our Result.
        // If there was no problem, print the record.
        // Otherwise, print the error message and quit the program.
        match result {
            Ok(record) => println!("{:?}", record),
            Err(err) => {
                println!("error reading CSV from <stdin>: {}", err);
                process::exit(1);
            }
        }
    }
}
```

If we run our program again, we'll still see an error message, but it is no
longer a panic message:

```text
$ cat invalid
header1,header2
foo,bar
quux,baz,foobar
$ ./target/debug/csvtutor < invalid
StringRecord { position: Some(Position { byte: 16, line: 2, record: 1 }), fields: ["foo", "bar"] }
error reading CSV from <stdin>: CSV error: record 2 (line: 3, byte: 24): found record with 3 fields, but the previous record has 2 fields
```

The second step for moving to recoverable errors is to put our CSV record loop
into a separate function. This function then has the option of *returning* an
error, which our `main` function can then inspect and decide what to do with.

```no_run
//tutorial-error-03.rs
extern crate csv;

use std::error::Error;
use std::io;
use std::process;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // Examine our Result.
        // If there was no problem, print the record.
        // Otherwise, convert our error to a Box<Error> and return it.
        match result {
            Err(err) => return Err(From::from(err)),
            Ok(record) => {
              println!("{:?}", record);
            }
        }
    }
    Ok(())
}
```

Our new function, `run`, has a return type of `Result<(), Box<Error>>`. In
simple terms, this says that `run` either returns nothing when successful, or
if an error occurred, it returns a `Box<Error>`, which stands for "any kind of
error." A `Box<Error>` is hard to inspect if we cared about the specific error
that occurred. But for our purposes, all we need to do is gracefully print an
error message and exit the program.

The third and final step is to replace our explicit `match` expression with a
special Rust language feature: the question mark.

```no_run
//tutorial-error-04.rs
extern crate csv;

use std::error::Error;
use std::io;
use std::process;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // This is effectively the same code as our `match` in the
        // previous example. In other words, `?` is syntactic sugar.
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
```

This last step shows how we can use the `?` to automatically forward errors
to our caller without having to do explicit case analysis with `match`
ourselves. We will use the `?` heavily throughout this tutorial, and it's
important to note that it can **only be used in functions that return
`Result`.**

We'll end this section with a word of caution: using `Box<Error>` as our error
type is the minimally acceptable thing we can do here. Namely, while it allows
our program to gracefully handle errors, it makes it hard for callers to
inspect the specific error condition that occurred. However, since this is a
tutorial on writing command line programs that do CSV parsing, we will consider
ourselves satisfied. If you'd like to know more, or are interested in writing
a library that handles CSV data, then you should check out my
[blog post on error handling](http://blog.burntsushi.net/rust-error-handling/).

With all that said, if all you're doing is writing a one-off program to do
CSV transformations, then using methods like `expect` and panicking when an
error occurs is a perfectly reasonable thing to do. Nevertheless, this tutorial
will endeavor to show idiomatic code.

# Reading CSV

Now that we've got you setup and covered basic error handling, it's time to do
what we came here to do: handle CSV data. We've already seen how to read
CSV data from `stdin`, but this section will cover how to read CSV data from
files and how to configure our CSV reader to data formatted with different
delimiters and quoting strategies.

First up, let's adapt the example we've been working with to accept a file
path argument instead of stdin.

```no_run
//tutorial-read-01.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

If you replace the contents of your `src/main.rs` file with the above code,
then you should be able to rebuild your project and try it out:

```text
$ cargo build
$ ./target/debug/csvtutor uspop.csv
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
# ... and much more
```

This example contains two new pieces of code:

1. Code for querying the positional arguments of your program. We put this code
   into its own funcation called `get_first_arg`. Our program expects a file
   path in the first position (which is indexed at `1`; the argument at index
   `0` is the executable name), so if one doesn't exist, then `get_first_arg`
   returns an error.
2. Code for opening a file. In `run`, we open a file using `File::open`. If
   there was a problem opening the file, we forward the error to the caller of
   `run` (which is `main` in this program). Note that we do *not* wrap the
   `File` in a buffer. The CSV reader does buffering internally, so there's
   no need for the caller to do it.

Now is a good time to introduce an alternate CSV reader constructor, which
makes it slightly more convenient to open CSV data from a file. That is,
instead of:

```ignore
let file_path = get_first_arg()?;
let file = File::open(file_path)?;
let mut rdr = csv::Reader::from_reader(file);
```

you can use:

```ignore
let file_path = get_first_arg()?;
let mut rdr = csv::Reader::from_path(file_path)?;
```

`csv::Reader::from_path` will open the file for you and return an error if
the file could not be opened.

## Reading headers

If you had a chance to look at the data inside `uspop.csv`, you would notice
that there is a header record that looks like this:

```text
City,State,Population,Latitude,Longitude
```

Now, if you look back at the output of the commands you've run so far, you'll
notice that the header record is never printed. Why is that? By default, the
CSV reader will interpret the first record in CSV data as a header, which
is typically distinct from the actual data in the records that follow.
Therefore, the header record is always skipped whenever you try to read or
iterate over the records in CSV data.

The CSV reader does not try to be smart about the header record and does
**not** employ any heuristics for automatically detecting whether the first
record is a header or not. Instead, if you don't want to treat the first record
as a header, you'll need to tell the CSV reader that there are no headers.

To configure a CSV reader to do this, we'll need to use a
[`ReaderBuilder`](../struct.ReaderBuilder.html)
to build a CSV reader with our desired configuration. Here's an example that
does just that. (Note that we've moved back to reading from `stdin`, since it
produces terser examples.)

```no_run
//tutorial-read-headers-01.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(io::stdin());
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

If you compile and run this program with our `uspop.csv` data, then you'll see
that the header record is now printed:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop.csv
StringRecord(["City", "State", "Population", "Latitude", "Longitude"])
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
```

If you ever need to access the header record directly, then you can use the
[`Reader::header`](../struct.Reader.html#method.headers)
method like so:

```no_run
//tutorial-read-headers-02.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    {
        // We nest this call in its own scope because of lifetimes.
        let headers = rdr.headers()?;
        println!("{:?}", headers);
    }
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    // We can ask for the headers at any time. There's no need to nest this
    // call in its own scope because we never try to borrow the reader again.
    let headers = rdr.headers()?;
    println!("{:?}", headers);
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

One interesting thing to note in this example is that we put the call to
`rdr.headers()` in its own scope. We do this because `rdr.headers()` returns
a *borrow* of the reader's internal header state. The nested scope in this
code allows the borrow to end before we try to iterate over the records. If
we didn't nest the call to `rdr.headers()` in its own scope, then the code
wouldn't compile because we cannot borrow the reader's headers at the same time
that we try to borrow the reader to iterate over its records.

Another way of solving this problem is to *clone* the header record:

```ignore
let headers = rdr.headers()?.clone();
```

This converts it from a borrow of the CSV reader to a new owned value. This
makes the code a bit easier to read, but at the cost of copying the header
record into a new allocation.

## Delimiters, quotes and variable length records

In this section we'll temporarily depart from our `uspop.csv` data set and
show how to read some CSV data that is a little less clean. This CSV data
uses `;` as a delimiter, escapes quotes with `\"` (instead of `""`) and has
records of varying length. Here's the data, which contains a list of WWE
wrestlers and the year they started, if it's known:

```text
$ cat strange.csv
"\"Hacksaw\" Jim Duggan";1987
"Bret \"Hit Man\" Hart";1984
# We're not sure when Rafael started, so omit the year.
Rafael Halperin
"\"Big Cat\" Ernie Ladd";1964
"\"Macho Man\" Randy Savage";1985
"Jake \"The Snake\" Roberts";1986
```

To read this CSV data, we'll want to do the following:

1. Disable headers, since this data has none.
2. Change the delimiter from `,` to `;`.
3. Change the quote strategy from doubled (e.g., `""`) to escaped (e.g., `\"`).
4. Permit flexible length records, since some omit the year.
5. Ignore lines beginning with a `#`.

All of this (and more!) can be configured with a
[`ReaderBuilder`](../struct.ReaderBuilder.html),
as seen in the following example:

```no_run
//tutorial-read-delimiter-01.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .double_quote(false)
        .escape(Some(b'\\'))
        .flexible(true)
        .comment(Some(b'#'))
        .from_reader(io::stdin());
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Now re-compile your project and try running the program on `strange.csv`:

```text
$ cargo build
$ ./target/debug/csvtutor < strange.csv
StringRecord(["\"Hacksaw\" Jim Duggan", "1987"])
StringRecord(["Bret \"Hit Man\" Hart", "1984"])
StringRecord(["Rafael Halperin"])
StringRecord(["\"Big Cat\" Ernie Ladd", "1964"])
StringRecord(["\"Macho Man\" Randy Savage", "1985"])
StringRecord(["Jake \"The Snake\" Roberts", "1986"])
```

You should feel encouraged to play around with the settings. Some interesting
things you might try:

1. If you remove the `escape` setting, notice that no CSV errors are reported.
   Instead, records are still parsed. This is a feature of the CSV parser. Even
   though it gets the data slightly wrong, it still provides a parse that you
   might be able to work with. This is a useful property given the messiness
   of real world CSV data.
2. If you remove the `delimiter` setting, parsing still succeeds, although
   every record has exactly one field.
3. If you remove the `flexible` setting, the reader will print the first two
   records (since they both have the same number of fields), but will return a
   parse error on the third record, since it has only one field.

This covers most of the things you might want to configure on your CSV reader,
although there are a few other knobs. For example, you can change the record
terminator from a new line to any other character. (By default, the terminator
is `CRLF`, which treats each of `\r\n`, `\r` and `\n` as single record
terminators.) For more details, see the documentation and examples for each of
the methods on
[`ReaderBuilder`](../struct.ReaderBuilder.html).

## Reading with Serde

One of the most convenient features of this crate is its support for
[Serde](https://serde.rs/).
Serde is a framework for automatically serializing and deserializing data into
Rust types. In simpler terms, that means instead of iterating over records
as an array of string fields, we can iterate over records of a specific type
of our choosing.

For example, let's take a look at some data from our `uspop.csv` file:

```text
City,State,Population,Latitude,Longitude
Davidsons Landing,AK,,65.2419444,-165.2716667
Kenai,AK,7610,60.5544444,-151.2583333
```

While some of these fields make sense as strings (`City`, `State`), other
fields look more like numbers. For example, `Population` looks like it contains
integers while `Latitude` and `Longitude` appear to contain decimals. If we
wanted to convert these fields to their "proper" types, then we need to do
a lot of manual work. This next example shows how.

```no_run
//tutorial-read-serde-01.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        let record = result?;

        let city = &record[0];
        let state = &record[1];
        // Some records are missing population counts, so if we can't
        // parse a number, treat the population count as missing instead
        // of returning an error.
        let pop: Option<u64> = record[2].parse().ok();
        // Lucky us! Latitudes and longitudes are available for every record.
        // Therefore, if one couldn't be parsed, return an error.
        let latitude: f64 = record[3].parse()?;
        let longitude: f64 = record[4].parse()?;

        println!(
            "city: {:?}, state: {:?}, \
             pop: {:?}, latitude: {:?}, longitude: {:?}",
            city, state, pop, latitude, longitude);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

The problem here is that we need to parse each individual field manually, which
can be labor intensive and repetitive. Serde, however, makes this process
automatic. For example, we can ask to deserialize every record into a tuple
type: `(String, String, Option<u64>, f64, f64)`.

```no_run
//tutorial-read-serde-02.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = (String, String, Option<u64>, f64, f64);

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    // Instead of creating an iterator with the `records` method, we create
    // an iterator with the `deserialize` method.
    for result in rdr.deserialize() {
        // We must tell Serde what type we want to deserialize into.
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Running this code should show similar output as previous examples:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop.csv
("Davidsons Landing", "AK", None, 65.2419444, -165.2716667)
("Kenai", "AK", Some(7610), 60.5544444, -151.2583333)
("Oakman", "AL", None, 33.7133333, -87.3886111)
# ... and much more
```

One of the downsides of using Serde this way is that the type you use must
match the order of fields as they appear in each record. This can be a pain
if your CSV data has a header record, since you might tend to think about each
field as a value of a particular named field rather than as a numbered field.
One way we might achieve this is to deserialize our record into a map type like
[`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
or
[`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html).
The next example shows how, and in particular, notice that the only thing that
changed from the last example is the definition of the `Record` type alias and
a new `use` statement that imports `HashMap` from the standard library:

```no_run
//tutorial-read-serde-03.rs
# extern crate csv;
#
use std::collections::HashMap;
# use std::error::Error;
# use std::io;
# use std::process;

// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = HashMap<String, String>;

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Running this program shows similar results as before, but each record is
printed as a map:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop.csv
{"City": "Davidsons Landing", "Latitude": "65.2419444", "State": "AK", "Population": "", "Longitude": "-165.2716667"}
{"City": "Kenai", "Population": "7610", "State": "AK", "Longitude": "-151.2583333", "Latitude": "60.5544444"}
{"State": "AL", "City": "Oakman", "Longitude": "-87.3886111", "Population": "", "Latitude": "33.7133333"}
```

This method works especially well if you need to read CSV data with header
records, but whose exact structure isn't known until your program runs.
However, in our case, we know the structure of the data in `uspop.csv`.
In particular, with the `HashMap` approach, we've lost the specific types
we had for each field in the previous example when we deserialized each record
into a `(String, String, Option<u64>, f64, f64)`. Is there a way to identify
fields by their corresponding header name *and* assign each field its own
unique type? The answer is yes, but we'll need to bring in a new crate called
`serde_derive` first. You can do that by adding this to the `[dependencies]`
section of your `Cargo.toml` file:

```text
serde = "1"
serde_derive = "1"
```

With these crates added to our project, we can now define our own custom struct
that represents our record. We then ask Serde to automatically write the glue
code required to populate our struct from a CSV record. The next example shows
how. Don't miss the new `extern crate` lines!

```no_run
//tutorial-read-serde-04.rs
extern crate csv;
extern crate serde;
// This lets us write `#[derive(Deserialize)]`.
#[macro_use]
extern crate serde_derive;

use std::error::Error;
use std::io;
use std::process;

// We don't need to derive `Debug` (which doesn't require Serde), but it's a
// good habit to do it for all your types.
//
// Notice that the field names in this struct are NOT in the same order as
// the fields in the CSV data!
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    population: Option<u64>,
    city: String,
    state: String,
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
        // Try this if you don't like each record smushed on one line:
        // println!("{:#?}", record);
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

Compile and run this program to see similar output as before:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop.csv
Record { latitude: 65.2419444, longitude: -165.2716667, population: None, city: "Davidsons Landing", state: "AK" }
Record { latitude: 60.5544444, longitude: -151.2583333, population: Some(7610), city: "Kenai", state: "AK" }
Record { latitude: 33.7133333, longitude: -87.3886111, population: None, city: "Oakman", state: "AL" }
```

Once again, we didn't need to change our `run` function at all: we're still
iterating over records using the `deserialize` iterator that we started with
in the beginning of this section. The only thing that changed in this example
was the definition of the `Record` type and a couple new `extern crate`
statements. Our `Record` type is now a custom struct that we defined instead
of a type alias, and as a result, Serde doesn't know how to deserialize it by
default. However, a special compiler plugin called `serde_derive` is available,
which will read your struct definition at compile time and generate code that
will deserialize a CSV record into a `Record` value. To see what happens if you
leave out the automatic derive, change `#[derive(Debug, Deserialize)]` to
`#[derive(Debug)]`.

One other thing worth mentioning in this example is the use of
`#[serde(rename_all = "PascalCase")]`. This directive helps Serde map your
struct's field names to the header names in the CSV data. If you recall, our
header record is:

```text
City,State,Population,Latitude,Longitude
```

Notice that each name is capitalized, but the fields in our struct are not. The
`#[serde(rename_all = "PascalCase")]` directive fixes that by interpreting each
field in `PascalCase`, where the first letter of the field is capitalized. If
we didn't tell Serde about the name remapping, then the program will quit with
an error:

```text
$ ./target/debug/csvtutor < uspop.csv
CSV deserialize error: record 1 (line: 2, byte: 41): missing field `latitude`
```

We could have fixed this through other means. For example, we could have used
capital letters in our field names:

```ignore
#[derive(Debug, Deserialize)]
struct Record {
    Latitude: f64,
    Longitude: f64,
    Population: Option<u64>,
    City: String,
    State: String,
}
```

However, this violates Rust naming style. (In fact, the Rust compiler
will even warn you that the names do not follow convention!)

Another way to fix this is to ask Serde to rename each field individually. This
is useful when there is no consistent name mapping from fields to header names:

```ignore
#[derive(Debug, Deserialize)]
struct Record {
    #[serde(rename = "Latitude")]
    latitude: f64,
    #[serde(rename = "Longitude")]
    longitude: f64,
    #[serde(rename = "Population")]
    population: Option<u64>,
    #[serde(rename = "City")]
    city: String,
    #[serde(rename = "State")]
    state: String,
}
```

To read more about renaming fields and about other Serde directives, please
consult the
[Serde documentation on attributes](https://serde.rs/attributes.html).

## Handling invalid data with Serde

In this section we will see a brief example of how to deal with data that isn't
clean. To do this exercise, we'll work with a slightly tweaked version of the
US population data we've been using throughout this tutorial. This version of
the data is slightly messier than what we've been using. You can get it like
so:

```text
$ curl -LO 'https://raw.githubusercontent.com/BurntSushi/rust-csv/rewrite/examples/data/uspop-null.csv'
```

Let's start by running our program from the previous section:

```no_run
//tutorial-read-serde-invalid-01.rs
# extern crate csv;
# #[macro_use]
# extern crate serde_derive;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    population: Option<u64>,
    city: String,
    state: String,
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Compile and run it on our messier data:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop-null.csv
Record { latitude: 65.2419444, longitude: -165.2716667, population: None, city: "Davidsons Landing", state: "AK" }
Record { latitude: 60.5544444, longitude: -151.2583333, population: Some(7610), city: "Kenai", state: "AK" }
Record { latitude: 33.7133333, longitude: -87.3886111, population: None, city: "Oakman", state: "AL" }
# ... more records
CSV deserialize error: record 42 (line: 43, byte: 1710): field 2: invalid digit found in string
```

Oops! What happened? The program printed several records, but stopped when it
tripped over a deserialization problem. The error message says that it found
an invalid digit in the field at index `2` (which is the `Population` field)
on line 43. What does line 43 look like?

```text
$ head -n 43 uspop-null.csv | tail -n1
Flint Springs,KY,NULL,37.3433333,-86.7136111
```

Ah! The third field (index `2`) is supposed to either be empty or contain a
population count. However, in this data, it seems that `NULL` sometimes appears
as a value, presumably to indicate that there is no count available.

The problem with our current program is that it fails to read this record
because it doesn't know how to deserialize a `NULL` string into an
`Option<u64>`. That is, a `Option<u64>` either corresponds to an empty field
or an integer.

To fix this, we tell Serde to convert any deserialization errors on this field
to a `None` value, as shown in this next example:

```no_run
//tutorial-read-serde-invalid-02.rs
# extern crate csv;
# #[macro_use]
# extern crate serde_derive;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    #[serde(deserialize_with = "csv::invalid_option")]
    population: Option<u64>,
    city: String,
    state: String,
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

If you compile and run this example, then it should run to completion just
like the other examples:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop-null.csv
Record { latitude: 65.2419444, longitude: -165.2716667, population: None, city: "Davidsons Landing", state: "AK" }
Record { latitude: 60.5544444, longitude: -151.2583333, population: Some(7610), city: "Kenai", state: "AK" }
Record { latitude: 33.7133333, longitude: -87.3886111, population: None, city: "Oakman", state: "AL" }
# ... and more
```

The only change in this example was adding this attribute to the `population`
field in our `Record` type:

```ignore
#[serde(deserialize_with = "csv::invalid_option")]
```

The
[`invalid_option`](../fn.invalid_option.html)
function is a generic helper function that does one very simple thing: when
applied to `Option` fields, it will convert any deserialization error into a
`None` value. This is useful when you need to work with messy CSV data.

# Writing CSV

In this section we'll show a few examples that write CSV data. Writing CSV data
tends to be a bit more straight-forward than reading CSV data, since you get to
control the output format.

Let's start with the most basic example: writing a few CSV records to `stdout`.

```no_run
//tutorial-write-01.rs
extern crate csv;

use std::error::Error;
use std::io;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    // Since we're writing records manually, we must explicitly write our
    // header record. A header record is written the same way that other
    // records are written.
    wtr.write_record(&["City", "State", "Population", "Latitude", "Longitude"])?;
    wtr.write_record(&["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])?;
    wtr.write_record(&["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])?;
    wtr.write_record(&["Oakman", "AL", "", "33.7133333", "-87.3886111"])?;

    // A CSV writer maintains an internal buffer, so it's important
    // to flush the buffer when you're done.
    wtr.flush()?;
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

Compiling and running this example results in CSV data being printed:

```text
$ cargo build
$ ./target/debug/csvtutor
City,State,Population,Latitude,Longitude
Davidsons Landing,AK,,65.2419444,-165.2716667
Kenai,AK,7610,60.5544444,-151.2583333
Oakman,AL,,33.7133333,-87.3886111
```

Before moving on, it's worth taking a closer look at the `write_record`
method. In this example, it looks rather simple, but if you're new to Rust then
its type signature might look a little daunting:

```ignore
pub fn write_record<I, T>(&mut self, record: I) -> csv::Result<()>
    where I: IntoIterator<Item=T>, T: AsRef<[u8]>
{
    // implementation elided
}
```

To understand the type signature, we can break it down piece by piece.

1. The method takes two parameters: `self` and `record`.
2. `self` is a special parameter that corresponds to the `Writer` itself.
3. `record` is the CSV record we'd like to write. Its type is `I`, which is
   a generic type.
4. In the method's `where` clause, the `I` type is contrained by the
   `IntoIterator<Item=T>` bound. What that means is that `I` must satisfy the
   `IntoIterator` trait. If you look at the documentation of the
   [`IntoIterator` trait](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html),
   then we can see that it describes types that can build iterators. In this
   case, we want an iterator that yields *another* generic type `T`, where
   `T` is the type of each field we want to write.
5. `T` also appears in the method's `where` clause, but its constraint is the
   `AsRef<[u8]>` bound. The `AsRef` trait is a way to describe zero cost
   conversions between types in Rust. In this case, the `[u8]` in `AsRef<[u8]>`
   means that we want to be able to *borrow* a slice of bytes from `T`.
   The CSV writer will take these bytes and write them as a single field.
   The `AsRef<[u8]>` bound is useful because types like `String`, `&str`,
   `Vec<u8>` and `&[u8]` all satisfy it.
6. Finally, the method returns a `csv::Result<()>`, which is short-hand for
   `Result<(), csv::Error>`. That means `write_record` either returns nothing
   on success or returns a `csv::Error` on failure.

Now, let's apply our new found understanding of the type signature of
`write_record`. If you recall, in our previous example, we used it like so:

```ignore
wtr.write_record(&["field 1", "field 2", "etc"])?;
```

So how do the types match up? Well, the type of each of our fields in this
code is `&'static str` (which is the type of a string literal in Rust). Since
we put them in a slice literal, the type of our parameter is
`&'static [&'static str]`, or more succinctly written as `&[&str]` without the
lifetime annotations. Since slices satisfy the `IntoIterator` bound and
strings satisfy the `AsRef<[u8]>` bound, this ends up being a legal call.

Here are a few more examples of ways you can call `write_record`:

```no_run
# use csv;
# let mut wtr = csv::Writer::from_writer(vec![]);
// A slice of byte strings.
wtr.write_record(&[b"a", b"b", b"c"]);
// A vector.
wtr.write_record(vec!["a", "b", "c"]);
// A string record.
wtr.write_record(&csv::StringRecord::from(vec!["a", "b", "c"]));
// A byte record.
wtr.write_record(&csv::ByteRecord::from(vec!["a", "b", "c"]));
```

Finally, the example above can be easily adapted to write to a file instead
of `stdout`:

```no_run
//tutorial-write-02.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let file_path = get_first_arg()?;
    let mut wtr = csv::Writer::from_path(file_path)?;

    wtr.write_record(&["City", "State", "Population", "Latitude", "Longitude"])?;
    wtr.write_record(&["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])?;
    wtr.write_record(&["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])?;
    wtr.write_record(&["Oakman", "AL", "", "33.7133333", "-87.3886111"])?;

    wtr.flush()?;
    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

## Writing tab separated values

In the previous section, we saw how to write some simple CSV data to `stdout`
that looked like this:

```text
City,State,Population,Latitude,Longitude
Davidsons Landing,AK,,65.2419444,-165.2716667
Kenai,AK,7610,60.5544444,-151.2583333
Oakman,AL,,33.7133333,-87.3886111
```

You might wonder to yourself: what's the point of using a CSV writer if the
data is so simple? Well, the benefit of a CSV writer is that it can handle all
types of data without sacrificing the integrity of your data. That is, it knows
when to quote fields that contain special CSV characters (like commas or new
lines) or escape literal quotes that appear in your data. The CSV writer can
also be easily configured to use different delimiters or quoting strategies.

In this section, we'll take a look a look at how to tweak some of the settings
on a CSV writer. In particular, we'll write TSV ("tab separated values")
instead of CSV, and we'll ask the CSV writer to quote all non-numeric fields.
Here's an example:

```no_run
//tutorial-write-delimiter-01.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(io::stdout());

    wtr.write_record(&["City", "State", "Population", "Latitude", "Longitude"])?;
    wtr.write_record(&["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])?;
    wtr.write_record(&["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])?;
    wtr.write_record(&["Oakman", "AL", "", "33.7133333", "-87.3886111"])?;

    wtr.flush()?;
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Compiling and running this example gives:

```text
$ cargo build
$ ./target/debug/csvtutor
"City"  "State" "Population"    "Latitude"      "Longitude"
"Davidsons Landing"     "AK"    ""      65.2419444      -165.2716667
"Kenai" "AK"    7610    60.5544444      -151.2583333
"Oakman"        "AL"    ""      33.7133333      -87.3886111
```

In this example, we used a new type
[`QuoteStyle`](../enum.QuoteStyle.html).
The `QuoteStyle` type represents the different quoting strategies available
to you. The default is to add quotes to fields only when necessary. This
probably works for most use cases, but you can also ask for quotes to always
be put around fields, to never be put around fields or to always be put around
non-numeric fields.

## Writing with Serde

Just like the CSV reader supports automatic deserialization into Rust types
with Serde, the CSV writer supports automatic serialization from Rust types
into CSV records using Serde. In this section, we'll learn how to use it.

As with reading, let's start by seeing how we can serialize a Rust tuple.

```no_run
//tutorial-write-serde-01.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    // We still need to write headers manually.
    wtr.write_record(&["City", "State", "Population", "Latitude", "Longitude"])?;

    // But now we can write records by providing a normal Rust value.
    //
    // Note that the odd `None::<u64>` syntax is required because `None` on
    // its own doesn't have a concrete type, but Serde needs a concrete type
    // in order to serialize it. That is, `None` has type `Option<T>` but
    // `None::<u64>` has type `Option<u64>`.
    wtr.serialize(("Davidsons Landing", "AK", None::<u64>, 65.2419444, -165.2716667))?;
    wtr.serialize(("Kenai", "AK", Some(7610), 60.5544444, -151.2583333))?;
    wtr.serialize(("Oakman", "AL", None::<u64>, 33.7133333, -87.3886111))?;

    wtr.flush()?;
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Compiling and running this program gives the expected output:

```text
$ cargo build
$ ./target/debug/csvtutor
City,State,Population,Latitude,Longitude
Davidsons Landing,AK,,65.2419444,-165.2716667
Kenai,AK,7610,60.5544444,-151.2583333
Oakman,AL,,33.7133333,-87.3886111
```

The key thing to note in the above example is the use of `serialize` instead
of `write_record` to write our data. In particular, `write_record` is used
when writing a simple record that contains string-like data only. On the other
hand, `serialize` is used when your data consists of more complex values like
numbers, floats or optional values. Of course, you could always convert the
complex values to strings and then use `write_record`, but Serde can do it for
you automatically.

As with reading, we can also serialize custom structs as CSV records. As a
bonus, the fields in a struct will automatically be written as a header
record!

To write custom structs as CSV records, we'll need to make use of the
`serde_derive` crate again. As in the
[previous section on reading with Serde](#reading-with-serde),
we'll need to add a couple crates to our `[dependencies]` section in our
`Cargo.toml` (if they aren't already there):

```text
serde = "1"
serde_derive = "1"
```

And we'll also need to add a couple extra `extern crate` statements to our
code, as shown in the example:

```no_run
//tutorial-write-serde-02.rs
extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::error::Error;
use std::io;
use std::process;

// Note that structs can derive both Serialize and Deserialize!
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Record<'a> {
    city: &'a str,
    state: &'a str,
    population: Option<u64>,
    latitude: f64,
    longitude: f64,
}

fn run() -> Result<(), Box<Error>> {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    wtr.serialize(Record {
        city: "Davidsons Landing",
        state: "AK",
        population: None,
        latitude: 65.2419444,
        longitude: -165.2716667,
    })?;
    wtr.serialize(Record {
        city: "Kenai",
        state: "AK",
        population: Some(7610),
        latitude: 60.5544444,
        longitude: -151.2583333,
    })?;
    wtr.serialize(Record {
        city: "Oakman",
        state: "AL",
        population: None,
        latitude: 33.7133333,
        longitude: -87.3886111,
    })?;

    wtr.flush()?;
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

Compiling and running this example has the same output as last time, even
though we didn't explicitly write a header record:

```text
$ cargo build
$ ./target/debug/csvtutor
City,State,Population,Latitude,Longitude
Davidsons Landing,AK,,65.2419444,-165.2716667
Kenai,AK,7610,60.5544444,-151.2583333
Oakman,AL,,33.7133333,-87.3886111
```

In this case, the `serialize` method noticed that we were writing a struct
with field names. When this happens, `serialize` will automatically write a
header record (only if no other records have been written) that consists of
the fields in the struct in the order in which they are defined. Note that
this behavior can be disabled with the
[`WriterBuilder::has_headers`](../struct.WriterBuilder.html#method.has_headers)
method.

It's also worth pointing out the use of a *lifetime parameter* in our `Record`
struct:

```ignore
struct Record<'a> {
    city: &'a str,
    state: &'a str,
    population: Option<u64>,
    latitude: f64,
    longitude: f64,
}
```

The `'a` lifetime parameter corresponds to the lifetime of the `city` and
`state` string slices. This says that the `Record` struct contains *borrowed*
data. We could have written our struct without borrowing any data, and
therefore, without any lifetime parameters:

```ignore
struct Record {
    city: String,
    state: String,
    population: Option<u64>,
    latitude: f64,
    longitude: f64,
}
```

However, since we had to replace our borrowed `&str` types with owned `String`
types, we're now forced to allocate a new `String` value for both of `city`
and `state` for every record that we write. There's no intrinsic problem with
doing that, but it might be a bit wasteful.

For more examples and more details on the rules for serialization, please see
the
[`Writer::serialize`](../struct.Writer.html#method.serialize)
method.

# Pipelining

In this section, we're going to cover a few examples that demonstrate programs
that take CSV data as input, and produce possibly transformed or filtered CSV
data as output. This shows how to write a complete program that efficiently
reads and writes CSV data. Rust is well positioned to perform this task, since
you'll get great performance with the convenience of a high level CSV library.

## Filter by search

The first example of CSV pipelining we'll look at is a simple filter. It takes
as input some CSV data on stdin and a single string query as its only
positional argument, and it will produce as output CSV data that only contains
rows with a field that matches the query.

```no_run
//tutorial-pipeline-search-01.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::io;
use std::process;

fn run() -> Result<(), Box<Error>> {
    // Get the query from the positional arguments.
    // If one doesn't exist, return an error.
    let query = match env::args().nth(1) {
        None => return Err(From::from("expected 1 argument, but got none")),
        Some(query) => query,
    };

    // Build CSV readers and writers to stdin and stdout, respectively.
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut wtr = csv::Writer::from_writer(io::stdout());

    // Before reading our data records, we should write the header record.
    wtr.write_record(rdr.headers()?)?;

    // Iterate over all the records in `rdr`, and write only records containing
    // `query` to `wtr`.
    for result in rdr.records() {
        let record = result?;
        if record.iter().any(|field| field == &query) {
            wtr.write_record(&record)?;
        }
    }

    // CSV writers use an internal buffer, so we should always flush when done.
    wtr.flush()?;
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

If we compile and run this program with a query of `MA` on `uspop.csv`, we'll
see that only one record matches:

```text
$ cargo build
$ ./csvtutor MA < uspop.csv
City,State,Population,Latitude,Longitude
Reading,MA,23441,42.5255556,-71.0958333
```

This example doesn't actually introduce anything new. It merely combines what
you've already learned about CSV readers and writers from previous sections.

Let's add a twist to this example. In the real world, you're often faced with
messy CSV data that might not be encoded correctly. One example you might come
across is CSV data encoded in
[Latin-1](https://en.wikipedia.org/wiki/ISO/IEC_8859-1).
Unfortunately, for the examples we've seen so far, our CSV reader assumes that
all of the data is UTF-8. Since all of the data we've worked on has been
ASCII---which is a subset of both Latin-1 and UTF-8---we haven't had any
problems. But let's introduce a slightly tweaked version of our `uspop.csv`
file that contains an encoding of a Latin-1 character that is invalid UTF-8.
You can get the data like so:

```text
$ curl -LO 'https://raw.githubusercontent.com/BurntSushi/rust-csv/rewrite/examples/data/uspop-latin1.csv'
```

Even though I've already given away the problem, let's see what happen when
we try to run our previous example on this new data:

```text
$ ./csvtutor MA < uspop-latin1.csv
City,State,Population,Latitude,Longitude
CSV parse error: record 3 (line 4, field: 0, byte: 125): invalid utf-8: invalid UTF-8 in field 0 near byte index 0
```

The error message tells us exactly what's wrong. Let's take a look at line 4
to see what we're dealing with:

```text
$ head -n4 uspop-latin1.csv | tail -n1
Õakman,AL,,33.7133333,-87.3886111
```

In this case, the very first character is the Latin-1 `Õ`, which is encoded as
the byte `0xD5`, which is in turn invalid UTF-8. So what do we do now that our
CSV parser has choked on our data? You have two choices. The first is to go in
and fix up your CSV data so that it's valid UTF-8. This is probably a good
idea anyway, and tools like `iconv` can help with the task of transcoding.
But if you can't or don't want to do that, then you can instead read CSV data
in a way that is mostly encoding agnostic (so long as ASCII is still a valid
subset). The trick is to use *byte records* instead of *string records*.

Thus far, we haven't actually talked much about the type of a record in this
library, but now is a good time to introduce them. There are two of them,
[`StringRecord`](../struct.StringRecord.html)
and
[`ByteRecord`](../struct.ByteRecord.html).
Each them represent a single record in CSV data, where a record is a sequence
of an arbitrary number of fields. The only difference between `StringRecord`
and `ByteRecord` is that `StringRecord` is guaranteed to be valid UTF-8, where
as `ByteRecord` contains arbitrary bytes.

Armed with that knowledge, we can now begin to understand why we saw an error
when we ran the last example on data that wasn't UTF-8. Namely, when we call
`records`, we get back an iterator of `StringRecord`. Since `StringRecord` is
guaranteed to be valid UTF-8, trying to build a `StringRecord` with invalid
UTF-8 will result in the error that we see.

All we need to do to make our example work is to switch from a `StringRecord`
to a `ByteRecord`. This means using `byte_records` to create our iterator
instead of `records`, and similarly using `byte_headers` instead of `headers`
if we think our header data might contain invalid UTF-8 as well. Here's the
change:

```no_run
//tutorial-pipeline-search-02.rs
# extern crate csv;
#
# use std::env;
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let query = match env::args().nth(1) {
        None => return Err(From::from("expected 1 argument, but got none")),
        Some(query) => query,
    };

    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut wtr = csv::Writer::from_writer(io::stdout());

    wtr.write_record(rdr.byte_headers()?)?;

    for result in rdr.byte_records() {
        let record = result?;
        // `query` is a `String` while `field` is now a `&[u8]`, so we'll
        // need to convert `query` to `&[u8]` before doing a comparison.
        if record.iter().any(|field| field == query.as_bytes()) {
            wtr.write_record(&record)?;
        }
    }

    wtr.flush()?;
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

Compiling and running this now yields the same results as our first example,
but this time it works on data that isn't valid UTF-8.

```text
$ cargo build
$ ./csvtutor MA < uspop-latin1.csv
City,State,Population,Latitude,Longitude
Reading,MA,23441,42.5255556,-71.0958333
```

## Filter by population count

In this section, we will show another example program that both reads and
writes CSV data, but instead of dealing with arbitrary records, we will use
Serde to deserialize and serialize records with specific types.

# Performance

## Amortizing allocations

## Serde and zero allocation

## CSV parsing without the standard library

# Closing thoughts
*/
