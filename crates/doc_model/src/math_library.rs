//! Ready-to-Use Inline Mathematical Function, Formula & Symbol Library (`v3.0.11` Foundation).
//!
//! Produces native multi-span inline `TextSpan`s (`SerifItalic` variables, upright functions and
//! operators, and real `<w:vertAlign>` `superscript` / `subscript` bounds) that sit naturally
//! inside normal prose paragraphs without breaking line height or requiring copy-paste.

use crate::{FontFamily, SpanStyle, TextSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathCategory {
    Core,
    Algebra,
    Calculus,
    Greek,
}

impl MathCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Algebra => "Algebra",
            Self::Calculus => "Calculus",
            Self::Greek => "Greek",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathPart {
    /// Upright mathematical operator, delimiter, or number (` = `, ` + `, `∑`, `∫`, `2`).
    Op(&'static str),
    /// Classical italic mathematical variable (`x`, `y`, `n`, `θ`).
    Var(&'static str),
    /// Upright mathematical function name (`sin`, `cos`, `lim`, `ln`, `log`).
    Fn(&'static str),
    /// Upright superscript run (`2`, `n−1`, `+∞`, `−1`).
    Sup(&'static str),
    /// Italic superscript variable (`n`, `k`, `x`, `iθ`).
    SupVar(&'static str),
    /// Upright subscript run (`0`, `1`, `i=1`, `max`).
    Sub(&'static str),
    /// Italic subscript variable (`i`, `k`, `n`, `x`).
    SubVar(&'static str),
}

pub struct MathTemplate {
    pub id: &'static str,
    pub category: MathCategory,
    pub title: &'static str,
    pub keywords: &'static str,
    pub preview: &'static str,
    pub parts: &'static [MathPart],
}

impl MathTemplate {
    /// Builds inline `TextSpan`s inheriting `font_size_pt` and `color` from the surrounding prose.
    pub fn to_inline_spans(&self, base: &SpanStyle) -> Vec<TextSpan> {
        let mut out: Vec<TextSpan> = Vec::with_capacity(self.parts.len());
        for part in self.parts {
            let mut st = SpanStyle {
                font_family: FontFamily::Serif,
                font_size_pt: base.font_size_pt,
                color: base.color,
                highlight: base.highlight,
                ..SpanStyle::default()
            };
            let txt = match *part {
                MathPart::Op(s) | MathPart::Fn(s) => s,
                MathPart::Var(s) => {
                    st.italic = true;
                    s
                }
                MathPart::Sup(s) => {
                    st.superscript = true;
                    s
                }
                MathPart::SupVar(s) => {
                    st.superscript = true;
                    st.italic = true;
                    s
                }
                MathPart::Sub(s) => {
                    st.subscript = true;
                    s
                }
                MathPart::SubVar(s) => {
                    st.subscript = true;
                    st.italic = true;
                    s
                }
            };
            if txt.is_empty() {
                continue;
            }
            if let Some(last) = out.last_mut() {
                if last.style == st {
                    last.text.push_str(txt);
                    continue;
                }
            }
            out.push(TextSpan::styled(txt.to_string(), st));
        }
        out
    }

    pub fn plain_text(&self) -> String {
        let mut s = String::new();
        for p in self.parts {
            match *p {
                MathPart::Op(t)
                | MathPart::Var(t)
                | MathPart::Fn(t)
                | MathPart::Sup(t)
                | MathPart::SupVar(t)
                | MathPart::Sub(t)
                | MathPart::SubVar(t) => s.push_str(t),
            }
        }
        s
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathSlashAction {
    OpenPicker(String),
    InsertTemplate(&'static str),
}

/// Parses `/math` or `/math <query_or_id>` on `Enter`.
pub fn parse_slash_math_command(line: &str) -> Option<MathSlashAction> {
    let trimmed = line.trim();
    if trimmed.eq_ignore_ascii_case("/math") || trimmed.eq_ignore_ascii_case("/formula") {
        return Some(MathSlashAction::OpenPicker(String::new()));
    }
    for prefix in ["/math ", "/formula "] {
        if trimmed.len() > prefix.len() && trimmed[..prefix.len()].eq_ignore_ascii_case(prefix) {
            let arg = trimmed[prefix.len()..].trim();
            if arg.is_empty() {
                return Some(MathSlashAction::OpenPicker(String::new()));
            }
            if let Some(exact) = find_math_template(arg) {
                return Some(MathSlashAction::InsertTemplate(exact.id));
            }
            let matches = filter_math_templates(arg);
            if let Some(first) = matches.first() {
                return Some(MathSlashAction::InsertTemplate(first.id));
            }
            return Some(MathSlashAction::OpenPicker(arg.to_string()));
        }
    }
    None
}

pub fn find_math_template(id: &str) -> Option<&'static MathTemplate> {
    MATH_TEMPLATES
        .iter()
        .find(|t| t.id.eq_ignore_ascii_case(id.trim()))
}

pub fn filter_math_templates(query: &str) -> Vec<&'static MathTemplate> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return MATH_TEMPLATES.iter().collect();
    }
    let terms: Vec<&str> = q.split_whitespace().collect();
    MATH_TEMPLATES
        .iter()
        .filter(|t| {
            let hay = format!(
                "{} {} {} {} {}",
                t.id.to_ascii_lowercase(),
                t.category.label().to_ascii_lowercase(),
                t.title.to_ascii_lowercase(),
                t.keywords.to_ascii_lowercase(),
                t.preview.to_ascii_lowercase()
            );
            terms.iter().all(|term| hay.contains(term))
        })
        .collect()
}

use MathCategory::{Algebra, Calculus, Core, Greek};
use MathPart::{Fn as MF, Op, Sub, SubVar, Sup, SupVar, Var};

pub const MATH_TEMPLATES: &[MathTemplate] = &[
    // ==================== 1. CORE INLINE BUILDING BLOCKS ====================
    MathTemplate {
        id: "pow",
        category: Core,
        title: "Power / Exponent",
        keywords: "power exponent superscript square cube x^n",
        preview: "xⁿ",
        parts: &[Var("x"), SupVar("n")],
    },
    MathTemplate {
        id: "sub",
        category: Core,
        title: "Indexed Variable (Subscript)",
        keywords: "subscript index sequence x_i a_k",
        preview: "xᵢ",
        parts: &[Var("x"), SubVar("i")],
    },
    MathTemplate {
        id: "sub_sup",
        category: Core,
        title: "Indexed Squared Term",
        keywords: "subscript superscript index square x_i^2",
        preview: "xᵢ²",
        parts: &[Var("x"), SubVar("i"), Sup("2")],
    },
    MathTemplate {
        id: "sqrt",
        category: Core,
        title: "Square Root Expression",
        keywords: "sqrt radical root pythagoras",
        preview: "√(x² + y²)",
        parts: &[Op("√("), Var("x"), Sup("2"), Op(" + "), Var("y"), Sup("2"), Op(")")],
    },
    MathTemplate {
        id: "nth_root",
        category: Core,
        title: "n-th Root",
        keywords: "nth root radical cube cbrt",
        preview: "ⁿ√x",
        parts: &[SupVar("n"), Op("√"), Var("x")],
    },
    MathTemplate {
        id: "frac_inline",
        category: Core,
        title: "Inline Rational Fraction",
        keywords: "fraction ratio quotient divide a/b",
        preview: "(a + b) / (c + d)",
        parts: &[
            Op("("), Var("a"), Op(" + "), Var("b"), Op(") / ("), Var("c"), Op(" + "), Var("d"), Op(")"),
        ],
    },
    MathTemplate {
        id: "sum_n",
        category: Core,
        title: "Summation (i = 1 to n)",
        keywords: "sum sigma series total",
        preview: "∑ᵢ₌₁ⁿ aᵢ",
        parts: &[Op("∑"), Sub("i=1"), SupVar("n"), Op(" "), Var("a"), SubVar("i")],
    },
    MathTemplate {
        id: "prod_n",
        category: Core,
        title: "Product (i = 1 to n)",
        keywords: "product pi multiply sequence",
        preview: "∏ᵢ₌₁ⁿ xᵢ",
        parts: &[Op("∏"), Sub("i=1"), SupVar("n"), Op(" "), Var("x"), SubVar("i")],
    },
    MathTemplate {
        id: "func_map",
        category: Core,
        title: "Function Mapping (Domain → Codomain)",
        keywords: "function map arrow domain range real",
        preview: "f : ℝⁿ → ℝ",
        parts: &[Var("f"), Op(" : ℝ"), SupVar("n"), Op(" → ℝ")],
    },
    MathTemplate {
        id: "interval_closed",
        category: Core,
        title: "Closed Real Interval",
        keywords: "interval element in [a, b] bounds",
        preview: "x ∈ [a, b]",
        parts: &[Var("x"), Op(" ∈ ["), Var("a"), Op(", "), Var("b"), Op("]")],
    },
    MathTemplate {
        id: "set_builder",
        category: Core,
        title: "Set-Builder Notation",
        keywords: "set braces condition real numbers",
        preview: "{ x ∈ ℝ | x ≥ 0 }",
        parts: &[Op("{ "), Var("x"), Op(" ∈ ℝ | "), Var("x"), Op(" ≥ 0 }")],
    },
    MathTemplate {
        id: "norm_l2",
        category: Core,
        title: "Euclidean L2 Norm",
        keywords: "norm magnitude vector length l2",
        preview: "‖x‖₂",
        parts: &[Op("‖"), Var("x"), Op("‖"), Sub("2")],
    },
    MathTemplate {
        id: "inner_prod",
        category: Core,
        title: "Inner / Angle Bracket Product",
        keywords: "inner dot product bra ket angle",
        preview: "⟨u, v⟩",
        parts: &[Op("⟨"), Var("u"), Op(", "), Var("v"), Op("⟩")],
    },
    MathTemplate {
        id: "floor_ceil",
        category: Core,
        title: "Floor & Ceiling Inequality",
        keywords: "floor ceiling integer round bounds",
        preview: "⌊x⌋ ≤ x ≤ ⌈x⌉",
        parts: &[Op("⌊"), Var("x"), Op("⌋ ≤ "), Var("x"), Op(" ≤ ⌈"), Var("x"), Op("⌉")],
    },

    // ==================== 2. ALGEBRA, SERIES & TRIGONOMETRY ====================
    MathTemplate {
        id: "quadratic",
        category: Algebra,
        title: "Quadratic Formula",
        keywords: "quadratic roots discriminant polynomial ax2+bx+c",
        preview: "x = (−b ± √(b² − 4ac)) / 2a",
        parts: &[
            Var("x"), Op(" = (−"), Var("b"), Op(" ± √("), Var("b"), Sup("2"),
            Op(" − 4"), Var("a"), Var("c"), Op(")) / 2"), Var("a"),
        ],
    },
    MathTemplate {
        id: "binomial_thm",
        category: Algebra,
        title: "Binomial Theorem",
        keywords: "binomial expansion newton choose",
        preview: "(a + b)ⁿ = ∑ₖ₌₀ⁿ C(n, k) aⁿ⁻ᵏ bᵏ",
        parts: &[
            Op("("), Var("a"), Op(" + "), Var("b"), Op(")"), SupVar("n"),
            Op(" = ∑"), Sub("k=0"), SupVar("n"), Op(" "), Var("C"), Op("("), Var("n"),
            Op(", "), Var("k"), Op(") "), Var("a"), Sup("n−k"), Op(" "), Var("b"), SupVar("k"),
        ],
    },
    MathTemplate {
        id: "diff_squares",
        category: Algebra,
        title: "Difference of Squares",
        keywords: "factor algebra squares a2-b2",
        preview: "a² − b² = (a − b)(a + b)",
        parts: &[
            Var("a"), Sup("2"), Op(" − "), Var("b"), Sup("2"),
            Op(" = ("), Var("a"), Op(" − "), Var("b"), Op(")("), Var("a"), Op(" + "), Var("b"), Op(")"),
        ],
    },
    MathTemplate {
        id: "diff_cubes",
        category: Algebra,
        title: "Difference of Cubes",
        keywords: "factor cubes a3-b3 polynomial",
        preview: "a³ − b³ = (a − b)(a² + ab + b²)",
        parts: &[
            Var("a"), Sup("3"), Op(" − "), Var("b"), Sup("3"), Op(" = ("),
            Var("a"), Op(" − "), Var("b"), Op(")("), Var("a"), Sup("2"), Op(" + "),
            Var("a"), Var("b"), Op(" + "), Var("b"), Sup("2"), Op(")"),
        ],
    },
    MathTemplate {
        id: "exp_growth",
        category: Algebra,
        title: "Exponential Growth / Decay",
        keywords: "exponential decay half life population e^kt",
        preview: "N(t) = N₀ eᵏᵗ",
        parts: &[
            Var("N"), Op("("), Var("t"), Op(") = "), Var("N"), Sub("0"),
            Op(" "), Var("e"), SupVar("kt"),
        ],
    },
    MathTemplate {
        id: "log_change_base",
        category: Algebra,
        title: "Logarithm Change of Base",
        keywords: "log ln base logarithm identity",
        preview: "log_b(x) = ln(x) / ln(b)",
        parts: &[
            MF("log"), SubVar("b"), Op("("), Var("x"), Op(") = "),
            MF("ln"), Op("("), Var("x"), Op(") / "), MF("ln"), Op("("), Var("b"), Op(")"),
        ],
    },
    MathTemplate {
        id: "log_rules",
        category: Algebra,
        title: "Logarithm Product & Power Rule",
        keywords: "ln product power logarithm",
        preview: "ln(xᵃ yᵇ) = a ln(x) + b ln(y)",
        parts: &[
            MF("ln"), Op("("), Var("x"), SupVar("a"), Op(" "), Var("y"), SupVar("b"),
            Op(") = "), Var("a"), Op(" "), MF("ln"), Op("("), Var("x"), Op(") + "),
            Var("b"), Op(" "), MF("ln"), Op("("), Var("y"), Op(")"),
        ],
    },
    MathTemplate {
        id: "arith_series",
        category: Algebra,
        title: "Arithmetic Series Sum",
        keywords: "arithmetic progression sum gauss",
        preview: "Sₙ = n(a₁ + aₙ) / 2",
        parts: &[
            Var("S"), SubVar("n"), Op(" = "), Var("n"), Op("("),
            Var("a"), Sub("1"), Op(" + "), Var("a"), SubVar("n"), Op(") / 2"),
        ],
    },
    MathTemplate {
        id: "geom_series",
        category: Algebra,
        title: "Geometric Series (Finite & Infinite)",
        keywords: "geometric series progression ratio converge",
        preview: "∑ₖ₌₀^∞ a rᵏ = a / (1 − r), |r| < 1",
        parts: &[
            Op("∑"), Sub("k=0"), Sup("∞"), Op(" "), Var("a"), Op(" "), Var("r"), SupVar("k"),
            Op(" = "), Var("a"), Op(" / (1 − "), Var("r"), Op("), |"), Var("r"), Op("| < 1"),
        ],
    },
    MathTemplate {
        id: "polynomial_n",
        category: Algebra,
        title: "Degree-n Polynomial",
        keywords: "polynomial coefficients degree algebra",
        preview: "P(x) = aₙxⁿ + … + a₁x + a₀",
        parts: &[
            Var("P"), Op("("), Var("x"), Op(") = "), Var("a"), SubVar("n"), Var("x"), SupVar("n"),
            Op(" + … + "), Var("a"), Sub("1"), Var("x"), Op(" + "), Var("a"), Sub("0"),
        ],
    },
    MathTemplate {
        id: "euler_formula",
        category: Algebra,
        title: "Euler's Complex Identity",
        keywords: "euler complex exponential cos sin imaginary",
        preview: "e^(iθ) = cos(θ) + i sin(θ)",
        parts: &[
            Var("e"), Sup("iθ"), Op(" = "), MF("cos"), Op("("), Var("θ"),
            Op(") + "), Var("i"), Op(" "), MF("sin"), Op("("), Var("θ"), Op(")"),
        ],
    },
    MathTemplate {
        id: "trig_pythagorean",
        category: Algebra,
        title: "Pythagorean Trigonometric Identity",
        keywords: "sin cos trig identity circle",
        preview: "sin²(θ) + cos²(θ) = 1",
        parts: &[
            MF("sin"), Sup("2"), Op("("), Var("θ"), Op(") + "),
            MF("cos"), Sup("2"), Op("("), Var("θ"), Op(") = 1"),
        ],
    },
    MathTemplate {
        id: "law_of_cosines",
        category: Algebra,
        title: "Law of Cosines",
        keywords: "triangle cosine geometry side angle",
        preview: "c² = a² + b² − 2ab cos(γ)",
        parts: &[
            Var("c"), Sup("2"), Op(" = "), Var("a"), Sup("2"), Op(" + "), Var("b"), Sup("2"),
            Op(" − 2"), Var("a"), Var("b"), Op(" "), MF("cos"), Op("("), Var("γ"), Op(")"),
        ],
    },
    MathTemplate {
        id: "triangle_ineq",
        category: Algebra,
        title: "Triangle Inequality",
        keywords: "absolute value norm inequality",
        preview: "|x + y| ≤ |x| + |y|",
        parts: &[Op("|"), Var("x"), Op(" + "), Var("y"), Op("| ≤ |"), Var("x"), Op("| + |"), Var("y"), Op("|")],
    },
    MathTemplate {
        id: "func_comp",
        category: Algebra,
        title: "Function Composition & Inverse",
        keywords: "composition fog inverse bijection",
        preview: "(f ∘ g)(x) = f(g(x)), f⁻¹(f(x)) = x",
        parts: &[
            Op("("), Var("f"), Op(" ∘ "), Var("g"), Op(")("), Var("x"), Op(") = "),
            Var("f"), Op("("), Var("g"), Op("("), Var("x"), Op(")), "),
            Var("f"), Sup("−1"), Op("("), Var("f"), Op("("), Var("x"), Op(")) = "), Var("x"),
        ],
    },

    // ==================== 3. CALCULUS & MATHEMATICAL ANALYSIS ====================
    MathTemplate {
        id: "limit_def",
        category: Calculus,
        title: "Limit as x Approaches a",
        keywords: "limit tends approaches continuity",
        preview: "lim_(x→a) f(x) = L",
        parts: &[MF("lim"), Sub("x→a"), Op(" "), Var("f"), Op("("), Var("x"), Op(") = "), Var("L")],
    },
    MathTemplate {
        id: "limit_euler_e",
        category: Calculus,
        title: "Euler's Constant Limit",
        keywords: "limit infinity compound euler e",
        preview: "lim_(n→∞) (1 + 1/n)ⁿ = e",
        parts: &[
            MF("lim"), Sub("n→∞"), Op(" (1 + 1/"), Var("n"), Op(")"), SupVar("n"), Op(" = "), Var("e"),
        ],
    },
    MathTemplate {
        id: "deriv_limit",
        category: Calculus,
        title: "First-Principles Derivative",
        keywords: "derivative difference quotient tangent slope",
        preview: "f′(x) = lim_(h→0) (f(x + h) − f(x)) / h",
        parts: &[
            Var("f"), Op("′("), Var("x"), Op(") = "), MF("lim"), Sub("h→0"),
            Op(" ("), Var("f"), Op("("), Var("x"), Op(" + "), Var("h"), Op(") − "),
            Var("f"), Op("("), Var("x"), Op(")) / "), Var("h"),
        ],
    },
    MathTemplate {
        id: "deriv_leibniz",
        category: Calculus,
        title: "Leibniz First & Second Derivative",
        keywords: "dy/dx d2y/dx2 derivative differential",
        preview: "dy/dx, d²y/dx²",
        parts: &[
            Op("d"), Var("y"), Op("/d"), Var("x"), Op(", d"), Sup("2"), Var("y"),
            Op("/d"), Var("x"), Sup("2"),
        ],
    },
    MathTemplate {
        id: "partial_deriv",
        category: Calculus,
        title: "Partial Derivative",
        keywords: "partial multivariable del df/dx",
        preview: "∂f / ∂xᵢ",
        parts: &[Op("∂"), Var("f"), Op(" / ∂"), Var("x"), SubVar("i")],
    },
    MathTemplate {
        id: "chain_rule",
        category: Calculus,
        title: "Chain Rule",
        keywords: "chain rule composite derivative calculus",
        preview: "d/dx [f(g(x))] = f′(g(x)) · g′(x)",
        parts: &[
            Op("d/d"), Var("x"), Op(" ["), Var("f"), Op("("), Var("g"), Op("("), Var("x"),
            Op("))] = "), Var("f"), Op("′("), Var("g"), Op("("), Var("x"), Op(")) · "),
            Var("g"), Op("′("), Var("x"), Op(")"),
        ],
    },
    MathTemplate {
        id: "gradient",
        category: Calculus,
        title: "Gradient Vector",
        keywords: "gradient nabla multivariable steepest",
        preview: "∇f(x) = (∂f/∂x₁, …, ∂f/∂xₙ)",
        parts: &[
            Op("∇"), Var("f"), Op("("), Var("x"), Op(") = (∂"), Var("f"), Op("/∂"), Var("x"),
            Sub("1"), Op(", …, ∂"), Var("f"), Op("/∂"), Var("x"), SubVar("n"), Op(")"),
        ],
    },
    MathTemplate {
        id: "laplacian",
        category: Calculus,
        title: "Divergence, Curl & Laplacian",
        keywords: "nabla divergence curl laplacian pde",
        preview: "∇ · F, ∇ × F, ∇²u = 0",
        parts: &[
            Op("∇ · "), Var("F"), Op(", ∇ × "), Var("F"), Op(", ∇"), Sup("2"), Var("u"), Op(" = 0"),
        ],
    },
    MathTemplate {
        id: "def_integral",
        category: Calculus,
        title: "Definite Integral (FTC)",
        keywords: "integral antiderivative area fundamental theorem",
        preview: "∫ₐᵇ f(x) dx = F(b) − F(a)",
        parts: &[
            Op("∫"), SubVar("a"), SupVar("b"), Op(" "), Var("f"), Op("("), Var("x"),
            Op(") d"), Var("x"), Op(" = "), Var("F"), Op("("), Var("b"), Op(") − "),
            Var("F"), Op("("), Var("a"), Op(")"),
        ],
    },
    MathTemplate {
        id: "ibp",
        category: Calculus,
        title: "Integration by Parts",
        keywords: "integration parts u dv uv",
        preview: "∫ u dv = uv − ∫ v du",
        parts: &[
            Op("∫ "), Var("u"), Op(" d"), Var("v"), Op(" = "), Var("u"), Var("v"),
            Op(" − ∫ "), Var("v"), Op(" d"), Var("u"),
        ],
    },
    MathTemplate {
        id: "double_integral",
        category: Calculus,
        title: "Double & Contour Integrals",
        keywords: "double surface contour integral flux",
        preview: "∬_D f(x, y) dA, ∮_C F · dr",
        parts: &[
            Op("∬"), SubVar("D"), Op(" "), Var("f"), Op("("), Var("x"), Op(", "), Var("y"),
            Op(") d"), Var("A"), Op(", ∮"), SubVar("C"), Op(" "), Var("F"), Op(" · d"), Var("r"),
        ],
    },
    MathTemplate {
        id: "taylor_series",
        category: Calculus,
        title: "Taylor Series Expansion",
        keywords: "taylor maclaurin power series approximation",
        preview: "f(x) = ∑ₖ₌₀^∞ (f⁽ᵏ⁾(a) / k!)(x − a)ᵏ",
        parts: &[
            Var("f"), Op("("), Var("x"), Op(") = ∑"), Sub("k=0"), Sup("∞"),
            Op(" ("), Var("f"), Sup("(k)"), Op("("), Var("a"), Op(") / "), Var("k"),
            Op("! )("), Var("x"), Op(" − "), Var("a"), Op(")"), SupVar("k"),
        ],
    },
    MathTemplate {
        id: "maclaurin_exp",
        category: Calculus,
        title: "Exponential Power Series",
        keywords: "maclaurin exp e^x factorial",
        preview: "eˣ = ∑ₖ₌₀^∞ xᵏ / k!",
        parts: &[
            Var("e"), SupVar("x"), Op(" = ∑"), Sub("k=0"), Sup("∞"),
            Op(" "), Var("x"), SupVar("k"), Op(" / "), Var("k"), Op("!"),
        ],
    },
    MathTemplate {
        id: "fourier_transform",
        category: Calculus,
        title: "Continuous Fourier Transform",
        keywords: "fourier frequency spectrum signal omega",
        preview: "F(ω) = ∫_(−∞)^(+∞) f(t) e^(−iωt) dt",
        parts: &[
            Var("F"), Op("("), Var("ω"), Op(") = ∫"), Sub("−∞"), Sup("+∞"),
            Op(" "), Var("f"), Op("("), Var("t"), Op(") "), Var("e"), Sup("−iωt"),
            Op(" d"), Var("t"),
        ],
    },

    // ==================== 4. GREEK VARIABLES, SETS & OPERATORS ====================
    MathTemplate {
        id: "alpha_beta_gamma",
        category: Greek,
        title: "α, β, γ, δ, ε (Greek Lowercase I)",
        keywords: "alpha beta gamma delta epsilon greek",
        preview: "α, β, γ, δ, ε",
        parts: &[Var("α"), Op(", "), Var("β"), Op(", "), Var("γ"), Op(", "), Var("δ"), Op(", "), Var("ε")],
    },
    MathTemplate {
        id: "alpha",
        category: Greek,
        title: "Alpha (α)",
        keywords: "alpha greek angle learning rate",
        preview: "α",
        parts: &[Var("α")],
    },
    MathTemplate {
        id: "beta",
        category: Greek,
        title: "Beta (β)",
        keywords: "beta greek coefficient momentum",
        preview: "β",
        parts: &[Var("β")],
    },
    MathTemplate {
        id: "gamma",
        category: Greek,
        title: "Gamma (γ / Γ)",
        keywords: "gamma greek discount factor lorentz",
        preview: "γ",
        parts: &[Var("γ")],
    },
    MathTemplate {
        id: "delta",
        category: Greek,
        title: "Delta Change (Δx / δ)",
        keywords: "delta difference change variation",
        preview: "Δx",
        parts: &[Op("Δ"), Var("x")],
    },
    MathTemplate {
        id: "epsilon",
        category: Greek,
        title: "Epsilon Neighborhood (ε > 0)",
        keywords: "epsilon small threshold ball analysis",
        preview: "ε > 0",
        parts: &[Var("ε"), Op(" > 0")],
    },
    MathTemplate {
        id: "theta",
        category: Greek,
        title: "Theta Parameter / Angle (θ / Θ)",
        keywords: "theta parameter angle complexity",
        preview: "θ",
        parts: &[Var("θ")],
    },
    MathTemplate {
        id: "lambda",
        category: Greek,
        title: "Lambda Eigenvalue / Rate (λ)",
        keywords: "lambda eigenvalue wavelength regularization",
        preview: "λ",
        parts: &[Var("λ")],
    },
    MathTemplate {
        id: "mu_sigma",
        category: Greek,
        title: "Mean & Standard Deviation (μ, σ²)",
        keywords: "mu sigma mean variance std normal",
        preview: "μ, σ²",
        parts: &[Var("μ"), Op(", "), Var("σ"), Sup("2")],
    },
    MathTemplate {
        id: "pi_tau_omega",
        category: Greek,
        title: "Pi, Phi, Psi, Omega (π, φ, ψ, ω, Ω)",
        keywords: "pi phi psi omega golden frequency",
        preview: "2π, φ, ψ, ω, Ω",
        parts: &[
            Op("2"), Var("π"), Op(", "), Var("φ"), Op(", "), Var("ψ"),
            Op(", "), Var("ω"), Op(", Ω"),
        ],
    },
    MathTemplate {
        id: "number_domains",
        category: Greek,
        title: "Standard Number Sets Chain",
        keywords: "natural integers rationals reals complex blackboard",
        preview: "ℕ ⊂ ℤ ⊂ ℚ ⊂ ℝ ⊂ ℂ",
        parts: &[Op("ℕ ⊂ ℤ ⊂ ℚ ⊂ ℝ ⊂ ℂ")],
    },
    MathTemplate {
        id: "reals_dim",
        category: Greek,
        title: "Euclidean Vector / Matrix Space",
        keywords: "real space Rn Rm×n tensor dimension",
        preview: "ℝ^(m×n)",
        parts: &[Op("ℝ"), Sup("m×n")],
    },
    MathTemplate {
        id: "quantifiers",
        category: Greek,
        title: "Universal & Existential Quantifiers",
        keywords: "forall exists unique logic predicate",
        preview: "∀x ∈ X, ∃y ∈ Y",
        parts: &[Op("∀"), Var("x"), Op(" ∈ "), Var("X"), Op(", ∃"), Var("y"), Op(" ∈ "), Var("Y")],
    },
    MathTemplate {
        id: "implication_iff",
        category: Greek,
        title: "Logical Implication & Equivalence",
        keywords: "implies iff equivalent iff arrow proof",
        preview: "P ⇒ Q ⇔ ¬Q ⇒ ¬P",
        parts: &[
            Var("P"), Op(" ⇒ "), Var("Q"), Op(" ⇔ ¬"), Var("Q"), Op(" ⇒ ¬"), Var("P"),
        ],
    },
    MathTemplate {
        id: "approx_congr",
        category: Greek,
        title: "Approximate, Equivalent & Non-Equal",
        keywords: "approx congruent neq leq geq proportional",
        preview: "x ≈ y, a ≡ b (mod m), u ≠ v",
        parts: &[
            Var("x"), Op(" ≈ "), Var("y"), Op(", "), Var("a"), Op(" ≡ "), Var("b"),
            Op(" ("), MF("mod"), Op(" "), Var("m"), Op("), "), Var("u"), Op(" ≠ "), Var("v"),
        ],
    },
    MathTemplate {
        id: "infinity_aleph",
        category: Greek,
        title: "Infinity, Partial, Nabla & Empty Set",
        keywords: "infinity empty set partial nabla hbar",
        preview: "∞, ∅, ∂, ∇, ℏ, ℵ₀",
        parts: &[Op("∞, ∅, ∂, ∇, ℏ, ℵ"), Sub("0")],
    },
];
