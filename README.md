# f*ck - Fields Combined with Columnar Keys

> Universal Columnar Data Merging Tool

**f*ck** is a powerful Rust-based data merging engine that empowers users to combine, clean, and transform messy tabular data through an intuitive DSL and visual interface.

## What is f*ck?

**f*ck** stands for **"fields combined with columnar keys"** - the core concept of merging data fields across multiple sources using columnar key relationships with intelligent merge policies.

### Key Features

- 🔗 **Smart Joins**: Dynamic column mapping between different data sources
- 📊 **Aggregation Policies**: Sum, Count, Average, Min, Max, FirstMatch
- 🎯 **Primary Key Logic**: OR/AND logic for complex key relationships  
- ⚡ **Lazy Evaluation**: Powered by Polars for efficient processing
- 🔄 **Incremental Computation**: Salsa-based caching for performance
- 🌐 **Multi-Modal**: CLI, Daemon+RPC, and WASM support
- 📝 **Visual DSL**: JSON-based query language

## Quick Start

### Installation

```bash
git clone https://github.com/your-repo/f-ck
cd f-ck
cargo build --release
```

### Basic Usage

1. **Prepare your data sources** (CSV, TSV, XLSX, SQLite)
2. **Create a query plan** (JSON DSL)
3. **Execute the merge**

```bash
# Preview results
./target/release/f-ck --query query.json --output result.csv --preview

# Write to file  
./target/release/f-ck --query query.json --output result.csv
```

## Example: Customer Order Analysis

### Input Files

**customers.csv**
```csv
id,name,email
1,John Doe,john@example.com
2,Jane Smith,jane@example.com
3,Bob Johnson,bob@example.com
```

**orders.csv**
```csv
customer_id,order_total,product
1,99.99,Widget A
2,149.50,Widget B
1,25.00,Widget C
```

### Query Plan (query.json)

```json
{
  "sources": [
    {
      "id": "customers",
      "path": "customers.csv", 
      "format": "csv"
    },
    {
      "id": "orders",
      "path": "orders.csv",
      "format": "csv"
    }
  ],
  "destination_schema": [
    {"name": "customer_id", "data_type": "Int64"},
    {"name": "customer_name", "data_type": "String"},
    {"name": "email", "data_type": "String"},
    {"name": "total_spent", "data_type": "Float64"}
  ],
  "primary_keys": {
    "logic": "or",
    "keys": ["customer_id"]
  },
  "mappings": [
    {
      "destination_field": "customer_id",
      "policy": {"type": "firstMatch", "priority": ["customers"]},
      "source_fields": [
        {"id": "cust_id", "source_file_id": "customers", "column_name": "id"},
        {"id": "order_cust_id", "source_file_id": "orders", "column_name": "customer_id"}
      ]
    },
    {
      "destination_field": "customer_name", 
      "policy": {"type": "firstMatch", "priority": ["customers"]},
      "source_fields": [
        {"id": "name", "source_file_id": "customers", "column_name": "name"}
      ]
    },
    {
      "destination_field": "email",
      "policy": {"type": "firstMatch", "priority": ["customers"]}, 
      "source_fields": [
        {"id": "email", "source_file_id": "customers", "column_name": "email"}
      ]
    },
    {
      "destination_field": "total_spent",
      "policy": {"type": "sum"},
      "source_fields": [
        {"id": "order_total", "source_file_id": "orders", "column_name": "order_total"}
      ]
    }
  ]
}
```

### Output

```csv
customer_id,customer_name,email,total_spent
1,John Doe,john@example.com,124.99
2,Jane Smith,jane@example.com,149.50
3,Bob Johnson,bob@example.com,0.0
```

## Merge Policies

| Policy | Description | Use Case |
|--------|-------------|----------|
| `FirstMatch` | Take first non-null value | Contact info, names |
| `Sum` | Add all values | Order totals, quantities |
| `Count` | Count non-null entries | Number of transactions |
| `Average` | Mean of all values | Average order size |
| `Min` | Minimum value | Earliest date, lowest price |
| `Max` | Maximum value | Latest date, highest price |

## CLI Options

```bash
f-ck [OPTIONS]

OPTIONS:
    -q, --query <FILE>     JSON file containing the query plan [required]
    -o, --output <FILE>    Output file path [required]
    -f, --format <FORMAT>  Output format: csv, tsv, xlsx, sqlite [default: csv]
    -p, --preview          Preview results without writing to file
    -l, --limit <N>        Limit preview to N rows
    -h, --help             Print help information
    -V, --version          Print version information
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Data Sources  │    │   Query DSL     │    │   Output        │
│                 │    │                 │    │                 │
│ • CSV/TSV       │───▶│ • Field Maps    │───▶│ • CSV/TSV       │
│ • XLSX          │    │ • Join Logic    │    │ • XLSX          │
│ • SQLite        │    │ • Merge Policy  │    │ • SQLite        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Core Components

- **DSL Engine**: JSON-based query planning and validation
- **Data Reader**: Multi-format input with Polars lazy evaluation  
- **Join Engine**: Dynamic column mapping and transitive closure
- **Aggregation Engine**: Group-by operations with merge policies
- **Output Writer**: Multi-format export with streaming

## Roadmap

### Phase 1: Core Engine ✅
- [x] Basic CSV join functionality
- [x] DSL query planning
- [x] Aggregation policies (sum, count, etc.)
- [x] CLI interface

### Phase 2: Advanced Features 🚧
- [ ] Salsa incremental computation
- [ ] WASM compilation support
- [ ] Transitive closure joins
- [ ] Type detection heuristics

### Phase 3: UI & Integration 📋
- [ ] Web-based visual interface
- [ ] Real-time preview system
- [ ] Data lineage tracking
- [ ] Recipe sharing

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit changes: `git commit -m 'Add amazing feature'`
4. Push to branch: `git push origin feature/amazing-feature`
5. Open a Pull Request

## Development

```bash
# Build and test
cargo build
cargo test

# Run with sample data
cargo run -- --query test_data/test_query.json --output result.csv --preview

# Check WASM compatibility (currently limited)
cargo check --target wasm32-unknown-unknown --lib
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Why "f*ck"?

The name represents both the frustration of working with messy data and the satisfaction of finally getting it clean. **f*ck** is about taking control of your data and making it work for you.

> *"f*ck around and find out... how clean your data can be."*