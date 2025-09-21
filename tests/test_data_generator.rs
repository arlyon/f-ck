use csv::Writer;
use fake::faker::address::en::{CityName, CountryCode, PostCode, StreetName};
use fake::faker::company::en::CompanyName;
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::{FirstName, LastName};
use fake::faker::phone_number::en::PhoneNumber;
use fake::{Fake, Faker};
use insta::assert_yaml_snapshot;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

pub fn generate_customers_csv<P: AsRef<Path>>(
    path: P,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write header
    wtr.write_record(&[
        "id",
        "name",
        "email",
        "phone",
        "address",
        "country",
        "post_code",
    ])?;

    for i in 1..=count {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();
        let full_name = format!("{} {}", first_name, last_name);

        wtr.write_record(&[
            i.to_string(),
            full_name,
            SafeEmail().fake::<String>(),
            PhoneNumber().fake::<String>(),
            StreetName().fake::<String>(),
            CountryCode().fake::<String>(),
            PostCode().fake::<String>(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn generate_orders_csv<P: AsRef<Path>>(
    path: P,
    count: usize,
    customer_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write header
    wtr.write_record(&["order_id", "customer_id", "product", "total", "date"])?;

    let products = vec![
        "Widget A",
        "Widget B",
        "Service Plan",
        "Premium Support",
        "Basic Package",
        "Advanced Kit",
        "Consultation",
        "Training",
    ];

    for i in 1..=count {
        let customer_id = (i % customer_count) + 1; // Distribute orders across customers
        let product = products[i % products.len()];
        let total: f64 = (10.0..500.0).fake();
        let date = format!("2024-{:02}-{:02}", (i % 12) + 1, (i % 28) + 1);

        wtr.write_record(&[
            i.to_string(),
            customer_id.to_string(),
            product.to_string(),
            format!("{:.2}", total),
            date,
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn generate_products_csv<P: AsRef<Path>>(
    path: P,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write header
    wtr.write_record(&["product_id", "name", "category", "price", "supplier"])?;

    let categories = vec![
        "Electronics",
        "Software",
        "Services",
        "Hardware",
        "Accessories",
    ];

    for i in 1..=count {
        let product_name = format!("Product {}", i);
        let category = categories[i % categories.len()];
        let price: f64 = (5.0..1000.0).fake();
        let supplier: String = CompanyName().fake();

        wtr.write_record(&[
            i.to_string(),
            product_name,
            category.to_string(),
            format!("{:.2}", price),
            supplier,
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn generate_web_activity_csv<P: AsRef<Path>>(
    path: P,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write header
    wtr.write_record(&[
        "session_id",
        "user_email",
        "page",
        "timestamp",
        "duration_seconds",
    ])?;

    let pages = vec![
        "/home",
        "/products",
        "/about",
        "/contact",
        "/pricing",
        "/blog",
        "/support",
    ];

    for i in 1..=count {
        let session_id = format!("sess_{}", i);
        let user_email: String = SafeEmail().fake();
        let page = pages[i % pages.len()];
        let timestamp = format!(
            "2024-01-{:02}T{:02}:{:02}:00Z",
            (i % 30) + 1,
            (i % 24),
            (i % 60)
        );
        let duration: u32 = (10..600).fake();

        wtr.write_record(&[
            session_id,
            user_email,
            page.to_string(),
            timestamp,
            duration.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn generate_mixed_types_csv<P: AsRef<Path>>(
    path: P,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write header
    wtr.write_record(&["id", "mixed_column", "type_indicator"])?;

    for i in 1..=count {
        let (value, type_name) = match i % 5 {
            0 => (SafeEmail().fake::<String>(), "email"),
            1 => (PhoneNumber().fake::<String>(), "phone"),
            2 => (format!("{}", (1..1000).fake::<i32>()), "integer"),
            3 => (format!("{:.2}", (1.0..100.0).fake::<f64>()), "float"),
            _ => (
                format!(
                    "{} {}",
                    FirstName().fake::<String>(),
                    LastName().fake::<String>()
                ),
                "name",
            ),
        };

        wtr.write_record(&[i.to_string(), value, type_name.to_string()])?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_all_datasets() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Generate all test datasets
        generate_customers_csv(dir_path.join("customers.csv"), 100).unwrap();
        generate_orders_csv(dir_path.join("orders.csv"), 500, 100).unwrap();
        generate_products_csv(dir_path.join("products.csv"), 50).unwrap();
        generate_web_activity_csv(dir_path.join("web_activity.csv"), 1000).unwrap();
        generate_mixed_types_csv(dir_path.join("mixed_types.csv"), 200).unwrap();

        // Verify files were created
        assert!(dir_path.join("customers.csv").exists());
        assert!(dir_path.join("orders.csv").exists());
        assert!(dir_path.join("products.csv").exists());
        assert!(dir_path.join("web_activity.csv").exists());
        assert!(dir_path.join("mixed_types.csv").exists());

        // Read and verify basic structure
        let customers_content = std::fs::read_to_string(dir_path.join("customers.csv")).unwrap();
        assert!(customers_content.contains("id,name,email"));
        assert!(customers_content.lines().count() > 100); // Header + 100 rows
    }
}
