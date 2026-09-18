//! Safe arithmetic formulas for calculated grid columns.
//!
//! Values use four decimal fixed-point places (`SCALE`). Multiplication and
//! division truncate toward zero after rescaling. Formulas contain no
//! evaluation of scripts, names, or external data: the host supplies values
//! for the referenced column IDs.

use std::collections::{BTreeMap, BTreeSet};

use crate::ColumnId;

/// Number of fixed-point units in one whole value.
pub const SCALE: i128 = 10_000;
const MAX_INPUT_BYTES: usize = 4096;
const MAX_NODES: usize = 256;
const MAX_DEPTH: usize = 32;
const MAX_CALCULATED_COLUMNS: usize = 64;
const MAX_FORMULA_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug)]
enum Expr {
    Literal(i128),
    Column(ColumnId),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
}

/// A parsed, bounded arithmetic formula.
#[derive(Clone, Debug)]
pub struct Formula {
    expr: Expr,
    referenced: Vec<ColumnId>,
}

/// A validated dependency graph of calculated columns.
#[derive(Clone, Debug)]
pub struct FormulaGraph {
    formulas: BTreeMap<ColumnId, Formula>,
    evaluation_order: Vec<ColumnId>,
}

impl FormulaGraph {
    /// Parse formulas, validate their references, and calculate a reusable
    /// topological evaluation order.
    pub fn new(base_columns: &[ColumnId], formulas: &[(ColumnId, String)]) -> Result<Self, String> {
        if formulas.len() > MAX_CALCULATED_COLUMNS {
            return Err("formula graph exceeds 64 calculated columns".into());
        }
        let source_bytes = formulas.iter().try_fold(0_usize, |total, (_, source)| {
            total.checked_add(source.len())
        });
        if source_bytes.is_none_or(|bytes| bytes > MAX_FORMULA_SOURCE_BYTES) {
            return Err("formula graph source exceeds 65536 bytes".into());
        }

        let mut base_ids = BTreeSet::new();
        for id in base_columns {
            if !base_ids.insert(*id) {
                return Err(format!("duplicate base column [{}]", id.0));
            }
        }

        let mut parsed = BTreeMap::new();
        for (id, source) in formulas {
            if base_ids.contains(id) {
                return Err(format!(
                    "derived column [{}] conflicts with a base column",
                    id.0
                ));
            }
            if parsed.contains_key(id) {
                return Err(format!("duplicate derived column [{}]", id.0));
            }
            let formula = Formula::parse(source)
                .map_err(|error| format!("derived column [{}]: {error}", id.0))?;
            parsed.insert(*id, formula);
        }

        for (id, formula) in &parsed {
            for referenced in formula.referenced_columns() {
                if !base_ids.contains(referenced) && !parsed.contains_key(referenced) {
                    return Err(format!(
                        "derived column [{}]: unknown column reference [{}]",
                        id.0, referenced.0
                    ));
                }
            }
        }

        let mut visit = BTreeMap::new();
        let mut evaluation_order = Vec::with_capacity(parsed.len());
        for id in parsed.keys().copied() {
            visit_column(id, &parsed, &mut visit, &mut evaluation_order)?;
        }
        Ok(Self {
            formulas: parsed,
            evaluation_order,
        })
    }

    /// Evaluate all calculated columns atomically for one row.
    pub fn evaluate(
        &self,
        resolve: impl Fn(ColumnId) -> Option<i128>,
    ) -> Result<BTreeMap<ColumnId, i128>, String> {
        let mut values = BTreeMap::new();
        for id in &self.evaluation_order {
            let formula = self
                .formulas
                .get(id)
                .ok_or_else(|| format!("derived column [{}]: formula is missing", id.0))?;
            let value = formula
                .evaluate(|referenced| {
                    values
                        .get(&referenced)
                        .copied()
                        .or_else(|| resolve(referenced))
                })
                .map_err(|error| format!("derived column [{}]: {error}", id.0))?;
            values.insert(*id, value);
        }
        Ok(values)
    }
}

fn visit_column(
    id: ColumnId,
    formulas: &BTreeMap<ColumnId, Formula>,
    visit: &mut BTreeMap<ColumnId, u8>,
    evaluation_order: &mut Vec<ColumnId>,
) -> Result<(), String> {
    match visit.get(&id) {
        Some(2) => return Ok(()),
        Some(1) => return Err(format!("derived column [{}]: dependency cycle", id.0)),
        _ => {}
    }
    visit.insert(id, 1);
    let formula = formulas
        .get(&id)
        .ok_or_else(|| format!("derived column [{}]: formula is missing", id.0))?;
    for referenced in formula.referenced_columns() {
        if formulas.contains_key(referenced) {
            visit_column(*referenced, formulas, visit, evaluation_order)
                .map_err(|error| format!("derived column [{}]: {error}", id.0))?;
        }
    }
    visit.insert(id, 2);
    evaluation_order.push(id);
    Ok(())
}

impl Formula {
    /// Parse literals, `[u64]` column references, `+ - * /`, unary minus and
    /// parentheses. Numeric literals have at most four fractional digits.
    pub fn parse(input: &str) -> Result<Self, String> {
        if input.len() > MAX_INPUT_BYTES {
            return Err("formula exceeds 4096 bytes".into());
        }
        let mut parser = Parser::new(input);
        let expr = parser.expression(0)?;
        parser.whitespace();
        if parser.peek().is_some() {
            return Err(parser.error("unexpected input"));
        }
        let mut referenced = parser.referenced;
        referenced.sort_unstable();
        referenced.dedup();
        Ok(Self { expr, referenced })
    }

    /// Column IDs used by this formula, sorted and deduplicated.
    pub fn referenced_columns(&self) -> &[ColumnId] {
        &self.referenced
    }

    /// Evaluate using fixed-point values supplied by the host.
    pub fn evaluate(&self, resolve: impl Fn(ColumnId) -> Option<i128>) -> Result<i128, String> {
        evaluate_expr(&self.expr, &resolve)
    }
}

fn evaluate_expr(expr: &Expr, resolve: &impl Fn(ColumnId) -> Option<i128>) -> Result<i128, String> {
    match expr {
        Expr::Literal(value) => Ok(*value),
        Expr::Column(id) => resolve(*id).ok_or_else(|| format!("missing field [{}]", id.0)),
        Expr::Neg(value) => evaluate_expr(value, resolve)?
            .checked_neg()
            .ok_or_else(|| "arithmetic overflow".into()),
        Expr::Add(left, right) => evaluate_expr(left, resolve)?
            .checked_add(evaluate_expr(right, resolve)?)
            .ok_or_else(|| "arithmetic overflow".into()),
        Expr::Sub(left, right) => evaluate_expr(left, resolve)?
            .checked_sub(evaluate_expr(right, resolve)?)
            .ok_or_else(|| "arithmetic overflow".into()),
        Expr::Mul(left, right) => {
            let product = evaluate_expr(left, resolve)?
                .checked_mul(evaluate_expr(right, resolve)?)
                .ok_or_else(|| "arithmetic overflow".to_string())?;
            product
                .checked_div(SCALE)
                .ok_or_else(|| "arithmetic overflow".into())
        }
        Expr::Div(left, right) => {
            let numerator = evaluate_expr(left, resolve)?
                .checked_mul(SCALE)
                .ok_or_else(|| "arithmetic overflow".to_string())?;
            let denominator = evaluate_expr(right, resolve)?;
            if denominator == 0 {
                return Err("division by zero".into());
            }
            numerator
                .checked_div(denominator)
                .ok_or_else(|| "arithmetic overflow".into())
        }
    }
}

struct Parser<'a> {
    input: &'a [u8],
    position: usize,
    nodes: usize,
    referenced: Vec<ColumnId>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
            nodes: 0,
            referenced: Vec::new(),
        }
    }

    fn expression(&mut self, depth: usize) -> Result<Expr, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("formula nesting exceeds depth 32"));
        }
        let mut left = self.term(depth)?;
        loop {
            self.whitespace();
            let operator = match self.peek() {
                Some(b'+') => b'+',
                Some(b'-') => b'-',
                _ => break,
            };
            self.position += 1;
            let right = self.term(depth)?;
            left = self.binary(operator, left, right)?;
        }
        Ok(left)
    }

    fn term(&mut self, depth: usize) -> Result<Expr, String> {
        let mut left = self.unary(depth)?;
        loop {
            self.whitespace();
            let operator = match self.peek() {
                Some(b'*') => b'*',
                Some(b'/') => b'/',
                _ => break,
            };
            self.position += 1;
            let right = self.unary(depth)?;
            left = self.binary(operator, left, right)?;
        }
        Ok(left)
    }

    fn unary(&mut self, depth: usize) -> Result<Expr, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("formula nesting exceeds depth 32"));
        }
        self.whitespace();
        if self.peek() == Some(b'-') {
            self.position += 1;
            self.node()?;
            return Ok(Expr::Neg(Box::new(self.unary(depth + 1)?)));
        }
        self.primary(depth)
    }

    fn primary(&mut self, depth: usize) -> Result<Expr, String> {
        self.whitespace();
        match self.peek() {
            Some(b'(') => {
                self.position += 1;
                let value = self.expression(depth + 1)?;
                self.whitespace();
                if self.peek() != Some(b')') {
                    return Err(self.error("expected ')'"));
                }
                self.position += 1;
                Ok(value)
            }
            Some(b'[') => self.column(),
            Some(byte) if byte.is_ascii_digit() => self.literal(),
            Some(_) => Err(self.error("expected number, column reference, or '('")),
            None => Err(self.error("expected expression")),
        }
    }

    fn literal(&mut self) -> Result<Expr, String> {
        let start = self.position;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.position += 1;
        }
        let whole = &self.input[start..self.position];
        let mut fraction = &[][..];
        if self.peek() == Some(b'.') {
            self.position += 1;
            let fraction_start = self.position;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.position += 1;
            }
            fraction = &self.input[fraction_start..self.position];
            if fraction.is_empty() || fraction.len() > 4 {
                return Err(self.error("literal needs 1 to 4 fractional digits"));
            }
        }
        let whole_value =
            parse_digits(whole).ok_or_else(|| self.error("invalid numeric literal"))?;
        let mut scaled = whole_value
            .checked_mul(SCALE)
            .ok_or_else(|| self.error("numeric literal overflow"))?;
        if !fraction.is_empty() {
            let fraction_value =
                parse_digits(fraction).ok_or_else(|| self.error("invalid numeric literal"))?;
            let factor = 10_i128.pow((4 - fraction.len()) as u32);
            scaled = scaled
                .checked_add(
                    fraction_value
                        .checked_mul(factor)
                        .ok_or_else(|| self.error("numeric literal overflow"))?,
                )
                .ok_or_else(|| self.error("numeric literal overflow"))?;
        }
        self.node()?;
        Ok(Expr::Literal(scaled))
    }

    fn column(&mut self) -> Result<Expr, String> {
        self.position += 1;
        let start = self.position;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.position += 1;
        }
        if start == self.position || self.peek() != Some(b']') {
            return Err(self.error("column reference must be [u64]"));
        }
        let id = parse_digits(&self.input[start..self.position])
            .ok_or_else(|| self.error("invalid column reference"))?;
        self.position += 1;
        let id =
            ColumnId(u64::try_from(id).map_err(|_| self.error("column reference must fit u64"))?);
        self.referenced.push(id);
        self.node()?;
        Ok(Expr::Column(id))
    }

    fn binary(&mut self, operator: u8, left: Expr, right: Expr) -> Result<Expr, String> {
        self.node()?;
        Ok(match operator {
            b'+' => Expr::Add(Box::new(left), Box::new(right)),
            b'-' => Expr::Sub(Box::new(left), Box::new(right)),
            b'*' => Expr::Mul(Box::new(left), Box::new(right)),
            b'/' => Expr::Div(Box::new(left), Box::new(right)),
            _ => return Err(self.error("invalid operator")),
        })
    }

    fn node(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            Err(self.error("formula exceeds 256 nodes"))
        } else {
            Ok(())
        }
    }

    fn whitespace(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
            self.position += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }
    fn error(&self, message: &str) -> String {
        format!("{} at byte {}", message, self.position)
    }
}

fn parse_digits(bytes: &[u8]) -> Option<i128> {
    let mut value = 0_i128;
    for byte in bytes {
        value = value
            .checked_mul(10)?
            .checked_add(i128::from(byte - b'0'))?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(source: &str) -> Result<i128, String> {
        Formula::parse(source)?.evaluate(|id| Some(i128::from(id.0) * SCALE))
    }

    #[test]
    fn precedence_and_references() {
        assert_eq!(eval("[2] + [3] * 2"), Ok(80_000));
    }
    #[test]
    fn fixed_point_and_truncation() {
        assert_eq!(eval("1.2345 * 2"), Ok(24_690));
        assert_eq!(eval("1 / 3"), Ok(3_333));
    }
    #[test]
    fn unary_negative_and_parentheses() {
        assert_eq!(eval("-(1.25 + 0.5)"), Ok(-17_500));
    }
    #[test]
    fn overflow_is_an_error() {
        assert!(Formula::parse(&(i128::MAX / SCALE + 1).to_string()).is_err());
    }
    #[test]
    fn division_zero_and_missing_field() {
        assert_eq!(eval("1 / 0"), Err("division by zero".into()));
        assert!(
            Formula::parse("[9]")
                .expect("parse")
                .evaluate(|_| None)
                .is_err()
        );
    }
    #[test]
    fn limits_are_enforced() {
        assert!(Formula::parse(&"1+".repeat(130)).is_err());
        assert!(Formula::parse(&format!("{}1", "(".repeat(33))).is_err());
        assert!(Formula::parse(&format!("{}1", "-".repeat(33))).is_err());
        assert!(Formula::parse(&"1".repeat(4097)).is_err());
    }
    #[test]
    fn unknown_field_is_reported_and_references_are_unique() {
        let formula = Formula::parse("[3]+[2]+[3]").expect("parse");
        assert_eq!(formula.referenced_columns(), &[ColumnId(2), ColumnId(3)]);
        assert!(formula.evaluate(|_| None).is_err());
    }

    #[test]
    fn graph_evaluates_chained_dependencies() {
        let graph = FormulaGraph::new(
            &[ColumnId(6)],
            &[
                (ColumnId(7), "[6] * 2".into()),
                (ColumnId(8), "[7] + 1".into()),
            ],
        )
        .expect("graph");
        let values = graph
            .evaluate(|id| (id == ColumnId(6)).then_some(30_000))
            .expect("evaluate");
        assert_eq!(values[&ColumnId(7)], 60_000);
        assert_eq!(values[&ColumnId(8)], 70_000);
    }

    #[test]
    fn graph_rejects_cycles_with_a_derived_id() {
        let error = FormulaGraph::new(
            &[],
            &[(ColumnId(7), "[8]".into()), (ColumnId(8), "[7]".into())],
        )
        .expect_err("cycle");
        assert!(error.contains("derived column ["));
        assert!(error.contains("cycle"));
    }

    #[test]
    fn graph_rejects_unknown_references_with_the_derived_id() {
        let error = FormulaGraph::new(&[ColumnId(6)], &[(ColumnId(7), "[9]".into())])
            .expect_err("unknown reference");
        assert!(error.contains("derived column [7]"));
        assert!(error.contains("unknown column reference [9]"));
    }

    #[test]
    fn graph_evaluation_is_atomic_on_overflow() {
        let graph = FormulaGraph::new(
            &[ColumnId(6)],
            &[
                (ColumnId(7), "[6] + 1".into()),
                (ColumnId(8), "[7] * [7]".into()),
            ],
        )
        .expect("graph");
        let error = graph
            .evaluate(|_| Some(i128::MAX - SCALE))
            .expect_err("overflow");
        assert!(error.contains("derived column [8]"));
        assert!(error.contains("overflow"));
    }
}
