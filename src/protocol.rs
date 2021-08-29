/*!
This module defines the core of the miette protocol: a series of types and traits
that you can implement to get access to miette's (and related library's) full
reporting and such features.
*/

use std::{
    fmt::{self, Display},
    fs,
    panic::Location,
};

use crate::MietteError;

/**
Adds rich metadata to your Error that can be used by [DiagnosticReportPrinter] to print
really nice and human-friendly error messages.
*/
pub trait Diagnostic: std::error::Error {
    /// Unique diagnostic code that can be used to look up more information
    /// about this Diagnostic. Ideally also globally unique, and documented in
    /// the toplevel crate's documentation for easy searching. Rust path
    /// format (`foo::bar::baz`) is recommended, but more classic codes like
    /// `E0123` or Enums will work just fine.
    fn code<'a>(&'a self) -> Box<dyn Display + 'a>;

    /// Diagnostic severity. This may be used by [DiagnosticReportPrinter]s to change the
    /// display format of this diagnostic.
    ///
    /// If `None`, reporters should treat this as [Severity::Error]
    fn severity(&self) -> Option<Severity> {
        None
    }

    /// Additional help text related to this Diagnostic. Do you have any
    /// advice for the poor soul who's just run into this issue?
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        None
    }

    /// URL to visit for a more details explanation/help about this Diagnostic.
    fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        None
    }

    /// Additional contextual snippets. This is typically used for adding
    /// marked-up source file output the way compilers often do.
    fn snippets<'a>(&'a self) -> Option<Box<dyn Iterator<Item = DiagnosticSnippet<'a>> + 'a>> {
        None
    }
}

impl std::error::Error for Box<dyn Diagnostic> {}

impl<T: Diagnostic + Send + Sync + 'static> From<T>
    for Box<dyn Diagnostic + Send + Sync + 'static>
{
    fn from(diag: T) -> Self {
        Box::new(diag)
    }
}

impl<T: Diagnostic + Send + Sync + 'static> From<T> for Box<dyn Diagnostic + Send + 'static> {
    fn from(diag: T) -> Self {
        Box::<dyn Diagnostic + Send + Sync>::from(diag)
    }
}

impl<T: Diagnostic + Send + Sync + 'static> From<T> for Box<dyn Diagnostic + 'static> {
    fn from(diag: T) -> Self {
        Box::<dyn Diagnostic + Send + Sync>::from(diag)
    }
}

/**
When used with `?`/`From`, this will wrap any Diagnostics and, when
formatted with `Debug`, will fetch the current [DiagnosticReportPrinter] and
use it to format the inner [Diagnostic].
*/
pub struct DiagnosticReport {
    diagnostic: Box<dyn Diagnostic + Send + Sync + 'static>,
}

impl DiagnosticReport {
    /// Return a reference to the inner [Diagnostic].
    pub fn inner(&self) -> &(dyn Diagnostic + Send + Sync + 'static) {
        &*self.diagnostic
    }
}

impl std::fmt::Debug for DiagnosticReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::get_printer().debug(&*self.diagnostic, f)
    }
}

impl<T: Diagnostic + Send + Sync + 'static> From<T> for DiagnosticReport {
    fn from(diagnostic: T) -> Self {
        DiagnosticReport {
            diagnostic: Box::new(diagnostic),
        }
    }
}

/// Convenience alias. This is intended to be used as the return type for `main()`
pub type DiagnosticResult<T> = Result<T, DiagnosticReport>;

/**
Protocol for [Diagnostic] handlers, which are responsible for actually printing out Diagnostics.

Blatantly based on [EyreHandler](https://docs.rs/eyre/0.6.5/eyre/trait.EyreHandler.html) (thanks, Jane!)
*/
pub trait DiagnosticReportPrinter: core::any::Any + Send + Sync {
    /// Define the report format.
    fn debug(
        &self,
        diagnostic: &(dyn Diagnostic),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result;

    /// Override for the `Display` format.
    fn display(
        &self,
        diagnostic: &(dyn Diagnostic),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        write!(f, "{}", diagnostic)?;
        Ok(())
    }
}

/**
[Diagnostic] severity. Intended to be used by [DiagnosticReportPrinter]s to change the
way different Diagnostics are displayed.
*/
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Severity {
    /// Critical failure. The program cannot continue.
    Error,
    /// Warning. Please take note.
    Warning,
    /// Just some help. Here's how you could be doing it better.
    Advice,
}

/**
Represents a readable source of some sort.

This trait is able to support simple Source types like [String]s, as well
as more involved types like indexes into centralized `SourceMap`-like types,
file handles, and even network streams.

If you can read it, you can source it,
and it's not necessary to read the whole thing--meaning you should be able to
support Sources which are gigabytes or larger in size.
*/
pub trait Source: std::fmt::Debug + Send + Sync {
    /// Read the bytes for a specific span from this Source.
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
    ) -> Result<Box<dyn SpanContents + 'a>, MietteError>;

    /// Optional name, usually a filename, for this source.
    fn name(&self) -> Option<String> {
        None
    }
}

/**
Contents of a [Source] covered by [SourceSpan].

Includes line and column information to optimize highlight calculations.
*/
pub trait SpanContents {
    /// Reference to the data inside the associated span, in bytes.
    fn data(&self) -> &[u8];
    /// The 0-indexed line in the associated [Source] where the data begins.
    fn line(&self) -> usize;
    /// The 0-indexed column in the associated [Source] where the data begins,
    /// relative to `line`.
    fn column(&self) -> usize;
}

/**
Basic implementation of the [SpanContents] trait, for convenience.
*/
#[derive(Clone, Debug)]
pub struct MietteSpanContents<'a> {
    /// Data from a [Source], in bytes.
    data: &'a [u8],
    // The 0-indexed line where the associated [SourceSpan] _starts_.
    line: usize,
    // The 0-indexed column where the associated [SourceSpan] _starts_.
    column: usize,
}

impl<'a> MietteSpanContents<'a> {
    /// Make a new [MietteSpanContents] object.
    pub fn new(data: &'a [u8], line: usize, column: usize) -> MietteSpanContents<'a> {
        MietteSpanContents { data, line, column }
    }
}

impl<'a> SpanContents for MietteSpanContents<'a> {
    fn data(&self) -> &[u8] {
        self.data
    }
    fn line(&self) -> usize {
        self.line
    }
    fn column(&self) -> usize {
        self.column
    }
}

/**
A snippet from a [Source] to be displayed with a message and possibly some highlights.
 */
#[derive(Clone, Debug)]
pub struct DiagnosticSnippet<'a> {
    /// Explanation of this specific diagnostic snippet.
    pub message: Option<String>,
    /// A [Source] that can be used to read the actual text of a source.
    pub source: &'a (dyn Source),
    /// The primary [SourceSpan] where this diagnostic is located.
    pub context: SourceSpan,
    /// Additional [SourceSpan]s that mark specific sections of the span, for
    /// example, to underline specific text within the larger span. They're
    /// paired with labels that should be applied to those sections.
    pub highlights: Option<Vec<(Option<String>, SourceSpan)>>,
}

/**
Span within a [Source] with an associated message.
*/
#[derive(Clone, Debug)]
pub struct SourceSpan {
    /// The start of the span.
    offset: SourceOffset,
    /// The total length of the span. Think of this as an offset from `start`.
    length: SourceOffset,
}

impl SourceSpan {
    /// Create a new [SourceSpan].
    pub fn new(start: SourceOffset, length: SourceOffset) -> Self {
        Self {
            offset: start,
            length,
        }
    }

    /// The absolute offset, in bytes, from the beginning of a [Source].
    pub fn offset(&self) -> usize {
        self.offset.offset()
    }

    /// Total length of the [SourceSpan], in bytes.
    pub fn len(&self) -> usize {
        self.length.offset()
    }

    /// Whether this [SourceSpan] has a length of zero. It may still be useful
    /// to point to a specific point.
    pub fn is_empty(&self) -> bool {
        self.length.offset() == 0
    }
}

impl From<(ByteOffset, ByteOffset)> for SourceSpan {
    fn from((start, len): (ByteOffset, ByteOffset)) -> Self {
        Self {
            offset: start.into(),
            length: len.into(),
        }
    }
}

impl From<(SourceOffset, SourceOffset)> for SourceSpan {
    fn from((start, len): (SourceOffset, SourceOffset)) -> Self {
        Self {
            offset: start,
            length: len,
        }
    }
}

/**
"Raw" type for the byte offset from the beginning of a [Source].
*/
pub type ByteOffset = usize;

/**
Newtype that represents the [ByteOffset] from the beginning of a [Source]
*/
#[derive(Clone, Copy, Debug)]
pub struct SourceOffset(ByteOffset);

impl SourceOffset {
    /// Actual byte offset.
    pub fn offset(&self) -> ByteOffset {
        self.0
    }

    /// Little utility to help convert line/column locations into
    /// miette-compatible Spans
    ///
    /// This function is infallible: Giving an out-of-range line/column pair
    /// will return the offset of the last byte in the source.
    pub fn from_location(source: impl AsRef<str>, loc_line: usize, loc_col: usize) -> Self {
        let mut line = 0usize;
        let mut col = 0usize;
        let mut offset = 0usize;
        for char in source.as_ref().chars() {
            if char == '\n' {
                col = 0;
                line += 1;
            } else {
                col += 1;
            }
            if line + 1 >= loc_line && col + 1 >= loc_col {
                break;
            }
            offset += char.len_utf8();
        }

        SourceOffset(offset)
    }

    /// Returns an offset for the _file_ location of wherever this function is
    /// called. If you want to get _that_ caller's location, mark this
    /// function's caller with `#[track_caller]` (and so on and so forth).
    ///
    /// Returns both the filename that was given and the offset of the caller
    /// as a SourceOffset
    ///
    /// Keep in mind that this fill only work if the file your Rust source
    /// file was compiled from is actually available at that location. If
    /// you're shipping binaries for your application, you'll want to ignore
    /// the Err case or otherwise report it.
    #[track_caller]
    pub fn from_current_location() -> Result<(String, Self), MietteError> {
        let loc = Location::caller();
        Ok((
            loc.file().into(),
            fs::read_to_string(loc.file())
                .map(|txt| Self::from_location(&txt, loc.line() as usize, loc.column() as usize))?,
        ))
    }
}

impl From<ByteOffset> for SourceOffset {
    fn from(bytes: ByteOffset) -> Self {
        SourceOffset(bytes)
    }
}
