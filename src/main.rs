fn main() {
    println!("This package is only for tests. Please run them with 'cargo test'");
}

#[cfg(test)]
mod test {
    use anyhow::{Context, anyhow};
    use assert_json_diff::assert_json_eq;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use sqlx::{Connection, Database, PgConnection, Row, postgres::PgRow};
    use std::{
        env,
        fs::{self, OpenOptions},
        io::{BufRead, BufReader},
        str::FromStr,
    };
    use test_case::test_case;

    use sqlx::Postgres;

    type Conn = <Postgres as Database>::Connection;

    async fn get_connection() -> anyhow::Result<Conn> {
        let url = env::var("PGURL")?;
        Ok(PgConnection::connect(&url).await?)
    }

    const PATHS_TABLE_GERMANY: &'static str = "spe_paths_europe_germany";
    const PATHS_TABLE_BRANDENBURG: &'static str = "spe_paths_europe_germany_brandenburg";
    const PATHS_TABLE_BERLIN: &'static str = "spe_paths_europe_germany_berlin";

    const ROADS_TABLE_GERMANY: &'static str = "spe_roads_europe_germany";
    const ROADS_TABLE_BRANDENBURG: &'static str = "spe_roads_europe_germany_brandenburg";
    const ROADS_TABLE_BERLIN: &'static str = "spe_roads_europe_germany_berlin";

    const ROADS_TABLE_NK: &'static str = "_sidepath_estimation_roads";
    const PATHS_TABLE_NK: &'static str = "_sidepath_estimation_paths";

    async fn table_difference_1(
        conn: &mut Conn,
        t1: &'static str,
        t2: &'static str,
    ) -> anyhow::Result<Option<i64>> {
        Ok(sqlx::query(&format!(
            r#"
            SELECT id FROM (
                SELECT id FROM {t1} EXCEPT SELECT id FROM {t2}
            )
            LIMIT 1
            "#
        ))
        .map(|r: PgRow| r.get(0))
        .fetch_optional(conn)
        .await?)
    }

    async fn assert_difference_exists(
        conn: &mut Conn,
        expected: bool,
        t1: &'static str,
        t2: &'static str,
    ) -> anyhow::Result<()> {
        let r = table_difference_1(conn, t1, t2).await?;
        let msg = || {
            if expected {
                format!("table {t1} is not a strict superset of {t2}")
            } else {
                format!("table {t1} is a strict subset of {t2}: {}", r.unwrap())
            }
        };
        assert!(r.is_some() == expected, "{}", msg());
        Ok(())
    }

    async fn assert_includes(
        conn: &mut Conn,
        t1: &'static str,
        t2: &'static str,
    ) -> anyhow::Result<()> {
        assert_difference_exists(conn, true, t1, t2).await?;
        assert_difference_exists(conn, false, t2, t1).await?;
        Ok(())
    }

    #[test_case(PATHS_TABLE_GERMANY, PATHS_TABLE_BRANDENBURG)]
    #[test_case(PATHS_TABLE_BRANDENBURG, PATHS_TABLE_BERLIN)]
    #[test_case(ROADS_TABLE_GERMANY, ROADS_TABLE_BRANDENBURG)]
    #[test_case(ROADS_TABLE_BRANDENBURG, ROADS_TABLE_BERLIN)]
    #[async_std::test]
    async fn test_table_includes(t1: &'static str, t2: &'static str) -> anyhow::Result<()> {
        let mut conn = get_connection().await?;
        assert_includes(&mut conn, t1, t2).await?;
        Ok(())
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct GeoJson {
        crs: serde_json::Value,
        features: Vec<Feature>,
    }

    impl GeoJson {
        fn srid(&self) -> Option<u64> {
            let name = self.crs.get("properties")?.get("name")?;
            str::parse(name.as_str()?.strip_prefix("urn:ogc:def:crs:EPSG::")?).ok()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Feature {
        properties: serde_json::Value,
        geometry: serde_json::Value,
    }

    impl Feature {
        // TODO: with error handling?
        fn id(&self) -> i64 {
            self.properties
                .get("id")
                .expect("geojons feature without 'id' property")
                .as_str()
                .expect("id property of geojson object is not a string")
                .strip_prefix("way/")
                .expect("id property does not start with 'way/'")
                .parse()
                .expect("stripped id is not a proper number")
        }
    }
    async fn import_geojson(
        conn: &mut Conn,
        geojson_path: &str,
        table: &'static str,
    ) -> anyhow::Result<()> {
        sqlx::raw_sql(&format!(
            r#"
            CREATE TEMP TABLE {table} (id bigint NOT NULL, tags jsonb, geom geometry);
            CREATE INDEX {table}_id_idx ON {table} (id ASC);
            CREATE INDEX {table}_geom_idx ON {table} (geom ASC);
            "#
        ))
        .execute(&mut *conn)
        .await
        .context("create")?;

        let geojson_string = fs::read_to_string(geojson_path)?;
        let geojson: GeoJson = serde_json::from_str(&geojson_string)?;

        // TODO: why don't I need this?
        let srid = geojson.srid().ok_or(anyhow!("Could not parse srid"))?;

        for feature in geojson.features {
            sqlx::query(&format!(
                "INSERT INTO {table} VALUES({})",
                "$1, $2, ST_GeomFromGeoJSON($3)"
            ))
            .bind(feature.id())
            .bind(json!({"tags": feature.properties}))
            .bind(feature.geometry)
            .execute(&mut *conn)
            .await?;
        }

        Ok(())
    }

    fn to_json_object(jsonl: &[serde_json::Value]) -> anyhow::Result<serde_json::Value> {
        let mut m = serde_json::Map::new();
        for json in jsonl {
            let line = json.as_array().ok_or(anyhow!("Should be an array"))?;
            let k = line
                .get(0)
                .expect("Should have elements")
                .as_number()
                .expect("Should be a number");
            let v = line.get(1).unwrap();
            m.insert(k.to_string(), v.clone());
        }

        Ok(serde_json::Value::Object(m))
    }

    fn read_sql_jsonl(path: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        BufReader::new(OpenOptions::new().read(true).open(path)?)
            .lines()
            .map(|l| -> anyhow::Result<serde_json::Value> { Ok(serde_json::Value::from_str(&l?)?) })
            .collect()
    }

    // TODO: don't use Result return type, but unwrap
    #[async_std::test]
    async fn test_against_nk_reference() -> anyhow::Result<()> {
        let mut conn = get_connection().await?;
        import_geojson(
            &mut conn,
            "test/reference-samples/berlin-nk-roads.geojson",
            ROADS_TABLE_NK,
        )
        .await?;
        import_geojson(
            &mut conn,
            "test/reference-samples/berlin-nk-paths.geojson",
            PATHS_TABLE_NK,
        )
        .await?;

        let sidepath_dict_jsonl = to_json_object(
            &sqlx::query("SELECT sidepath_dict_jsonl(100.0, 22.0)")
                .map(|r: PgRow| {
                    serde_json::Value::from_str(r.get(0))
                        .expect("sidepath_dict should return json rows")
                })
                .fetch_all(&mut conn)
                .await?,
        )?;
        let expected = to_json_object(&read_sql_jsonl(
            "test/reference-samples/sidepath_dict-berlin-nk-reference.sqljsonl",
        )?)?;
        assert_json_eq!(sidepath_dict_jsonl, expected);

        Ok(())
    }
    // TODO: port unit tests from pyton (CQI-repo; individual functions)
    // TODO: port tests for is_sidepath_no and is_sidepath_yes from tmp-check-brandenburg.sql
    // TODO: test that "latest" corresponds to the actual latest file (file with the latest date)
}
