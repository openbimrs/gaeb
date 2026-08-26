# GAEB business validation

Business validation is an optional layer over the byte-lossless document model. It does not replace XSD validation and never repairs source bytes.

## Classification contract

The catalog contains exactly 18 checks:

- **3 conformance errors** (`GAEB-BR-*`): narrowly scoped to release tuples for which retained GAEB evidence supports an error classification. Any emitted error makes the report invalid.
- **15 interoperability advisories** (`GAEB-LINT-*`): useful consistency checks whose normative basis or phase breadth is incomplete. They always remain warnings and never invalidate a report.

The catalog and severity lookup are centralized in `openbim-gaeb/src/business/mod.rs`. Tests require 18 unique identifiers, exactly 3 errors, and exactly 15 warnings.

## Error rules

| ID | Scope | Behavior |
|---|---|---|
| `GAEB-BR-X84-31-001` | namespace `http://www.gaeb.de/GAEB_DA_XML/200407`, declared version `3.1`, date `2009-12`, phase `84` | `Award/CTR` is required. |
| `GAEB-BR-X84-31-002` | same exact tuple | every `MarkupItem` requires `ITMarkup`. |
| `GAEB-BR-COST-001` | GAEB 3.3 date `2021-05`, with a DA50/DA51 namespace coherent with declared phase `50.1`, `50.2`, `51.1`, or `51.2` | a cost element used as a billing element must be terminal and may not be referenced as a component. |

Contradictory namespace/declaration tuples, unsupported version dates, and GAEB 3.4 beta documents do not trigger these errors. Regression tests pin these fail-closed applicability conditions.

## Advisory rules

| ID | Implemented scope |
|---|---|
| `GAEB-LINT-BOQ-001` | warns when a numeric breakdown is declared but `RNoPart` contains non-digits. |
| `GAEB-LINT-BOQ-002` | warns when the composed outline key exceeds fourteen characters. |
| `GAEB-LINT-BOQ-003` | compares `BoQBkdn`, category, and item outline signatures for a coherent X83→X84 pair. |
| `GAEB-LINT-PRICE-001` | compares item total with commercially rounded `Qty × UP` after item-level `DiscountPcnt`. |
| `GAEB-LINT-PRICE-002` | checks that each item supplies the number of unit-price components declared by its nearest containing BoQ. |
| `GAEB-LINT-PRICE-003` | compares the bounded unit-price-component sum with `UP`. |
| `GAEB-LINT-TOTAL-001` | compares declared net total with the sum of item totals. |
| `GAEB-LINT-TOTAL-002` | compares gross total with net plus VAT amount. |
| `GAEB-LINT-X84-001` | checks protected item presence, outline/quantity fields, and text for a coherent X83→X84 pair. |
| `GAEB-LINT-X84-002` | compares project VAT for a coherent X83→X84 pair. |
| `GAEB-LINT-QTY-001` | compares X31 quantity-calculation results with referenced LV item quantities for a coherent release pair. |
| `GAEB-LINT-TRADE-001` | compares order-item numbering for a coherent customer/contractor order pair. |
| `GAEB-LINT-TRADE-002` | checks dependent trade-price characteristic fields. |
| `GAEB-LINT-TEXT-001` | requires every supplied X84 `TextComplement` to reference a `MarkLbl` completion slot designated by the baseline X83 document. |
| `GAEB-LINT-DESCR-001` | compares description identity and content for a coherent X83→X84 pair. |

Pair validation first requires matching declared version and version date, matching namespace release segments, and a phase coherent with each document's namespace. Cross-release or internally contradictory pairs intentionally produce no pair lints rather than false evidence.

## Resource bounds

- XML nesting is rejected above 256 elements by the lossless parser and by the XSD document-shape preflight.
- `NoUPComps` is parsed as a bounded unsigned count and compared with the six GAEB `UPComp1`…`UPComp6` fields before any component traversal. Arbitrarily large declarations produce one warning instead of caller-controlled work.
- component declarations are resolved only from `BoQInfo` in the nearest containing BoQ; item-local, outer, sibling, and document-global declarations are never reused.
- decimal arithmetic uses bounded arbitrary-precision integers with exact fixed scales and explicit commercial rounding. Up to 4,096 decimal digits are computed exactly; values or intermediate scales beyond that resource budget fail closed with the applicable advisory instead of skipping validation. Item discounts are applied before the final two-decimal comparison.

## Executable evidence

Focused suites:

```sh
cargo +1.85.0 test -p openbim-gaeb --test business_validation
cargo +1.85.0 test -p openbim-gaeb --test business_rule_pairs
```

Coverage includes:

- positive and negative behavior for all 18 catalog entries;
- the 3-error/15-warning severity split;
- exact namespace/version/date/phase applicability for error rules;
- contradictory and cross-release tuples;
- multiple BoQs with different unit-price-component declarations;
- item discounts, arbitrary-precision values, and commercial rounding;
- description IDs, baseline-designated `MarkLbl` completion slots, and BoQ-breakdown changes;
- the exact six-component boundary: six is accepted, seven and larger declarations are rejected before traversal.

Retained mutation probes demonstrate that removing text filtering, component lookup, component budgeting, and warning-severity dispatch causes focused gates to fail. Business checks remain advisory unless the catalog explicitly identifies one of the three evidence-backed errors above.
