Product Requirements Document: f*ck (Universal Columnar Data Merging Tool)

    Version: 0.6

    Status: Draft

    Date: 2025-09-20

1. Introduction / Overview

This document outlines the requirements for the Universal Columnar Data Merging Tool, colloquially known as f*ck. The product consists of a powerful Rust-based core engine and an intuitive web-based user interface. Its primary purpose is to empower non-technical users to perform complex data cleaning, merging, and transformation tasks that are currently difficult or impossible without specialized tools or developer assistance.

Users will visually map and merge data from various tabular sources (like CSVs). The tool's key differentiator is its intelligent, graph-based merge logic (transitive closure) that can resolve entities across multiple files and keys, combined with a user interface that makes this complexity transparent and easy to debug.
2. Goals / Objectives

    Primary Goal: Enable non-technical users to independently merge and clean complex, messy tabular data with confidence. The user should feel empowered to "f*ck their data" — combine fields with columnar keys in powerful ways.

    Secondary Goal: Drive viral adoption through an exceptionally intuitive and powerful user experience. The tool should be so effective that users will naturally share it with colleagues.

    Technical Goal: Create a robust, lazy-evaluated, stream-processing engine that can be used headlessly via a CLI (f-ck). The engine's architecture will be built for high-performance, incremental computation to provide near-instantaneous feedback on query changes.

3. Target Audience / User Personas

    Primary Persona: "The Data-Driven Analyst"

        Role: Marketing Operations, Sales Analyst, Business Analyst, etc.

        Behavior: Frequently works with data exported from various systems (e.g., Salesforce, Google Analytics, internal databases) into spreadsheets. They are comfortable with Excel/Google Sheets but lack SQL or programming skills.

        Pain Points: Existing tools are either too simple (can't handle multi-file, multi-key joins) or too complex (require coding). They spend hours manually copying, pasting, and using VLOOKUP, a process that is slow, error-prone, and not repeatable.

4. User Stories / Use Cases
4.1. Primary Web UI Use Case

As a Marketing Analyst, I want to merge three separate CSV files containing customer data so that I can create a single, clean list for an email campaign.

    I open the f*ck web application and drag my three CSV files (contacts.csv, orders.csv, web_activity.csv) onto the canvas.

    The tool automatically scans each file and displays them as nodes, showing the columns (fields) within each one. It intelligently detects that a "contact_info" column in one file contains both emails and phone numbers and visually splits them into two sub-nodes for clarity.

    I decide what my final, merged file should look like by dragging fields from the source nodes into a "Destination" area on the canvas. I drag email and phone_number into a special "Primary Key" section.

    I then map the source data. I drag a link from the email field in contacts.csv to my destination email field. I do the same for order_total from orders.csv.

    When I map the address field, which exists in two of my source files, the UI prompts me to set a merge policy. I choose the First Match policy and specify that the web_activity.csv address should be prioritized.

    As I build these mappings, a "Preview" drawer at the bottom of the screen updates in real-time, showing both successfully resolved records and a list of "unresolved" records that couldn't be merged, with clear reasons why (e.g., "Type Mismatch on order_total").

    I click on a merged record in the preview to understand its origin. The UI highlights all the source rows and values from the original files that were considered to produce the final, merged row.

    Once I'm satisfied, I click "Download" and receive a clean, merged CSV file. I also choose to "Save Recipe" to re-run this same merge process in the future.

4.2. Power User CLI Use Case

As a Developer, I want to programmatically merge a set of files using a predefined mapping so that I can automate a data cleaning pipeline.

    I create an inputs.json file defining the paths to my source CSVs.

    I create a query.json file (which I may have initially designed and exported from the web UI) that defines the destination schema, primary keys, source-to-destination mappings, and merge policies.

    I run the command: f-ck --inputs inputs.json --query query.json --output final_merge.sqlite

    The core engine lazily evaluates the merge and streams the result directly to a new SQLite database file.

5. Functional Requirements

ID


Requirement


Details

FN-1


Data I/O


The system must support reading from and writing to the following formats: CSV, TSV, XLSX, SQLite. The set of output formats must match the input formats.

FN-2


Heuristic Type Detection


The engine must automatically detect the following common data types within columns: Email, Phone Number, Date, Country Code, Post Code, Name, Text, Integer, Float.

FN-3


Visual Mapping Canvas


The UI shall provide a drag-and-drop interface (react-flow based) for users to visually map source fields to destination fields.

FN-4


Merge Engine Logic


The core merge logic must be based on Transitive Closure. If A is linked to B via email and B is linked to C via phone, the engine shall treat A, B, and C as part of the same entity.

FN-5


Merge Policies


Users must be able to set per-column merge policies, including: First Match (with user-defined source priority), Sum, Count, Average, Min, Max.

FN-6


Real-time Preview


The UI must provide a real-time preview of both resolved and unresolved data that updates as the user modifies the mapping.

FN-7


Unresolved Data Handling


A record or field shall be flagged as "unresolved" if: 1. A lookup key finds no match. 2. A mapped field is empty in the source. 3. A type mismatch occurs during an operation (e.g., Sum on text).

FN-8


CLI Interface


The engine must be executable via a CLI (f-ck), accepting JSON files for input sources and the merge query.

FN-9


Configuration Export


Users must be able to save/export their merge mapping configuration (the "recipe") from the web UI, which will be in the same format as the CLI's query.json.
6. Non-Functional Requirements

ID


Requirement


Details

NFN-1


Usability


The web UI must be intuitive enough for the target non-technical user to perform a multi-file merge in their first session without documentation.

NFN-2


Performance


All data processing must be lazy-evaluated. UI interactions and previews should feel instantaneous, even on large datasets (e.g., multiple files of 500k+ rows).

NFN-3


Scalability


The initial version should comfortably handle merging up to 5 source files, with individual files up to 1GB or 1 million rows.

NFN-4


Security


All data processing must happen client-side or on a secure, isolated server instance. No user data shall be stored permanently on the server beyond the session duration.
7. Design Considerations / Mockups

    The core mapping interface will be built using react-flow or a similar library to represent data sources and mappings as nodes and edges.

    Key Design Challenge: Visually representing the transitive closure logic. When a user connects A -> B and then B -> C, the UI must provide a clear visual indication that A and C are now implicitly linked.

        Mermaid Diagram of Logic:

        graph TD
            A(Source A) -- Email --> B(Source B);
            B -- Phone --> C(Source C);
            A -.-> C_implicit(Source C);
            style C_implicit fill:#fff,stroke:#f00,stroke-width:2px,stroke-dasharray: 5 5

    Key Design Feature: The "data consideration" view. When a user clicks on a cell in the final merged preview, the UI must clearly show all the source values from all source files that were considered in that cell's computation. This is the primary mechanism for debugging and building trust.

8. Success Metrics

    Adoption & Virality:

        Number of active weekly users.

        Number of saved/shared merge "recipes".

        Net Promoter Score (NPS) or user satisfaction surveys.

    Engagement & Value:

        Average time to complete the first successful merge.

        Average number of source files used in a merge job.

        Percentage of users who save their merge configuration for reuse.

9. Architectural Strategy

Following a detailed analysis of incremental computation models and Rust query engines, the project will adopt Pattern B: The Salsa-Wrapped Execution Approach.

This architecture uses the Salsa framework to manage the execution of our core query engine (built with Polars/DataFusion). Salsa, a demand-driven incremental computation framework, will act as the "brains" of the engine, tracking dependencies between data sources and the final query result.

Why this approach?
This pattern is ideal for our interactive, user-driven application. It allows us to:

    Use Familiar, Powerful APIs: We can leverage the highly expressive and ergonomic DataFrame and SQL APIs from Polars and DataFusion to define our complex merging logic.

    Automate Caching and Invalidation: Salsa automatically handles the complexity of tracking which data has changed and what needs to be recomputed, simplifying development.

    Achieve High Interactivity: When a user changes a source file or a mapping rule, only the parts of the computation that are directly affected will be re-run. Unchanged data and intermediate results will be reused from Salsa's cache, making the UI feel extremely responsive.

In practice, this means when a user updates a single CSV file, only the initial parsing of that file is re-executed. The final merge query will then run again, but it will use the new data for the changed file and the cached, already-processed data for all other files, providing the performance of an incremental system with the development model of a traditional query engine.
10. Open Questions / Future Considerations

    Address Normalization: Future versions should include specialized logic for normalizing and merging physical addresses.

    Additional Data Sources: Plan for integration with data sources beyond file uploads (e.g., direct connections to PostgreSQL, Salesforce, Google Sheets).

    Performance Benchmarking: Define specific performance targets for the initial scan and real-time preview updates on benchmark datasets.
