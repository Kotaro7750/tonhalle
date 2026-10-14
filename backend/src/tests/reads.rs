//! Verifies music browsing and pagination through HTTP requests.

use super::*;

#[tokio::test]
async fn music_api_reads_sample_audio_in_the_contract_format() {
    let releases = sample_releases();

    // The provenance inventory is an independent expectation, never an indexing input.
    let inventory: Value =
        serde_json::from_str(include_str!("../../../sample/sources.json")).unwrap();

    let formats = inventory["releases"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|release| release["tracks"].as_array().unwrap())
        .map(|track| track["audio_properties"]["media_type"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        formats,
        [
            "audio/wav",
            "audio/flac",
            "audio/aac",
            "audio/mp4",
            "audio/ogg",
            "audio/mpeg"
        ]
        .into_iter()
        .collect()
    );

    let app = router(Arc::new(InMemoryMetadataIndex::new(releases)));

    let (status, list) = request(app.clone(), "/api/v0/releases").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["items"].as_array().unwrap().len(), 3);
    assert!(list.get("nextCursor").is_none());

    let ids = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|release| release["id"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));

    for expected in inventory["releases"].as_array().unwrap() {
        let id = expected["release_id"].as_str().unwrap();
        let (status, release) = request(app.clone(), &format!("/api/v0/releases/{id}")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(release["id"], id);
        assert_eq!(release["title"], expected["title"]);
        assert_eq!(release["releaseDate"], "2026-10-13");
        assert_eq!(
            release["artistCredits"].as_array().unwrap().len(),
            expected["artists"].as_array().unwrap().len()
        );

        for (credit, name) in release["artistCredits"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["artists"].as_array().unwrap())
        {
            assert_eq!(&credit["name"], name);
            assert_eq!(credit["joinPhrase"], Value::Null);
            assert!(
                credit["artistId"]
                    .as_str()
                    .unwrap()
                    .parse::<ArtistId>()
                    .is_ok()
            );
        }

        let summaries = release["tracks"].as_array().unwrap();

        assert_eq!(
            summaries.len(),
            expected["tracks"].as_array().unwrap().len()
        );
        assert!(summaries.windows(2).all(|pair| (
            pair[0]["discNumber"].as_u64(),
            pair[0]["trackNumber"].as_u64()
        ) <= (
            pair[1]["discNumber"].as_u64(),
            pair[1]["trackNumber"].as_u64()
        )));

        for (summary, expected_track) in
            summaries.iter().zip(expected["tracks"].as_array().unwrap())
        {
            assert_eq!(summary["id"], expected_track["track_id"]);

            let (status, track) = request(
                app.clone(),
                &format!("/api/v0/tracks/{}", summary["id"].as_str().unwrap()),
            )
            .await;

            assert_eq!(status, StatusCode::OK);

            for (key, value) in summary.as_object().unwrap() {
                assert_eq!(&track[key], value);
            }

            assert_eq!(track["title"], expected_track["title"]);
            assert_eq!(track["durationMs"], expected_track["duration_ms"]);
            assert_eq!(track["discNumber"], expected_track["disc_number"]);
            assert_eq!(track["trackNumber"], expected_track["track_number"]);

            let audio = &track["originalAudioFile"];

            let properties = &expected_track["audio_properties"];
            assert_eq!(
                audio,
                &json!({"id": expected_track["audio_file_id"], "mediaType": properties["media_type"], "playbackUrl": format!("https://media.example.com/music/{}", expected_track["path"].as_str().unwrap()), "sampleRate": properties["sample_rate"], "bitDepth": properties["bit_depth"], "channels": properties["channels"], "bitrate": properties["bitrate"]})
            );
        }
    }
}

#[tokio::test]
async fn list_releases_paginates_without_duplicates_when_limit_changes() {
    let app = router(Arc::new(InMemoryMetadataIndex::new(sample_releases())));

    let (_, all) = request(app.clone(), "/api/v0/releases").await;
    let (_, first) = request(app.clone(), "/api/v0/releases?limit=1").await;

    assert_eq!(first["items"].as_array().unwrap().len(), 1);

    let (_, rest) = request(
        app.clone(),
        &format!(
            "/api/v0/releases?limit=100&cursor={}",
            first["nextCursor"].as_str().unwrap()
        ),
    )
    .await;

    assert!(rest.get("nextCursor").is_none());

    let mut combined = first["items"].as_array().unwrap().clone();
    combined.extend(rest["items"].as_array().unwrap().clone());

    assert_eq!(combined, *all["items"].as_array().unwrap());

    let mut cursor = None;
    let mut ids = Vec::new();

    loop {
        let uri = cursor.map_or("/api/v0/releases?limit=1".to_string(), |cursor| {
            format!("/api/v0/releases?limit=1&cursor={cursor}")
        });
        let (_, page) = request(app.clone(), &uri).await;

        ids.extend(
            page["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["id"].clone()),
        );

        cursor = page["nextCursor"].as_str().map(str::to_string);
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(ids.len(), 3);
    assert_eq!(
        ids,
        all["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].clone())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn list_releases_defaults_to_thirty_and_rejects_unknown_cursor_positions() {
    let make_release = |i| {
        let mut source =
            crate::test_support::MetadataFixture::from(&crate::test_support::sample_metadata());

        source.release_title = format!("Pagination Release {i}");

        crate::test_support::sample_release(&source.build())
    };

    let app = router(Arc::new(InMemoryMetadataIndex::new(
        (0..120).map(make_release).collect(),
    )));

    let (_, first) = request(app.clone(), "/api/v0/releases").await;

    assert_eq!(first["items"].as_array().unwrap().len(), 30);

    let (_, rest) = request(
        app.clone(),
        &format!(
            "/api/v0/releases?limit=100&cursor={}",
            first["nextCursor"].as_str().unwrap()
        ),
    )
    .await;

    assert_eq!(rest["items"].as_array().unwrap().len(), 90);

    assert!(rest.get("nextCursor").is_none());

    let other_index = router(Arc::new(InMemoryMetadataIndex::new(
        (120..122).map(make_release).collect(),
    )));

    let (_, other_page) = request(other_index, "/api/v0/releases?limit=1").await;
    let cursor = other_page["nextCursor"].as_str().unwrap();

    assert_eq!(
        request(app, &format!("/api/v0/releases?cursor={cursor}"))
            .await
            .0,
        StatusCode::BAD_REQUEST,
    );
}

#[tokio::test]
async fn list_releases_returns_an_empty_page_without_a_cursor() {
    let (_, empty) = request(
        router(Arc::new(InMemoryMetadataIndex::new(vec![]))),
        "/api/v0/releases",
    )
    .await;

    assert_eq!(empty, json!({"items": []}));
}

#[tokio::test]
async fn music_api_indexes_supported_formats_with_detected_types_and_optional_bit_depth() {
    use crate::test_support::sample_fixture;
    use lofty::{
        config::WriteOptions,
        file::FileType,
        tag::{ItemKey, ItemValue, Tag, TagExt, TagItem},
    };

    let fixture = sample_fixture();
    let release_id = fixture.build_release_metadata().unwrap().id().clone();
    for (extension, bytes, media_type, bit_depth) in [
        (
            "flac",
            include_bytes!("../../../test_fixtures/tone.flac").as_slice(),
            "audio/flac",
            json!(16),
        ),
        (
            "aac",
            include_bytes!("../../../test_fixtures/tone.aac").as_slice(),
            "audio/aac",
            Value::Null,
        ),
        (
            "m4a",
            include_bytes!("../../../test_fixtures/tone.m4a").as_slice(),
            "audio/mp4",
            Value::Null,
        ),
        (
            "ogg",
            include_bytes!("../../../test_fixtures/tone.ogg").as_slice(),
            "audio/ogg",
            Value::Null,
        ),
        (
            "opus",
            include_bytes!("../../../test_fixtures/tone.opus").as_slice(),
            "audio/ogg",
            Value::Null,
        ),
        (
            "spx",
            include_bytes!("../../../test_fixtures/tone.spx").as_slice(),
            "audio/ogg",
            Value::Null,
        ),
        (
            "mp3",
            include_bytes!("../../../test_fixtures/tone.mp3").as_slice(),
            "audio/mpeg",
            Value::Null,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join(release_id.to_string());
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join(format!(
            "{}.{}",
            fixture.audio_file.id,
            extension.to_uppercase()
        ));
        std::fs::write(&path, bytes).unwrap();
        let mut tag = Tag::new(FileType::from_ext(extension).unwrap().primary_tag_type());
        for (key, value) in [
            (ItemKey::AlbumTitle, fixture.release_title.as_str()),
            (ItemKey::ReleaseDate, fixture.release_date.as_str()),
            (ItemKey::AlbumArtist, "Example Artist"),
            (ItemKey::TrackTitle, fixture.track_title.as_str()),
            (ItemKey::DiscNumber, "1"),
            (ItemKey::TrackNumber, "1"),
        ] {
            tag.insert_text(key, value.to_owned());
        }
        for name in ["First Artist", "Second Artist"] {
            tag.push(TagItem::new(
                ItemKey::TrackArtist,
                ItemValue::Text(name.into()),
            ));
        }
        tag.save_to_path(&path, WriteOptions::new()).unwrap();

        let storage = FileSystemMusicStorage::new(
            root.path().to_owned(),
            "https://media.example.com/".parse().unwrap(),
        )
        .unwrap();
        let releases = BuildMetadataIndexUseCase::new(Arc::new(storage))
            .execute()
            .unwrap();
        assert_eq!(releases.len(), 1, "{extension}");
        let app = router(Arc::new(InMemoryMetadataIndex::new(releases)));
        let (status, release) =
            request(app.clone(), &format!("/api/v0/releases/{release_id}")).await;
        assert_eq!(status, StatusCode::OK);
        let track_id = release["tracks"][0]["id"].as_str().unwrap();
        let (status, track) = request(app, &format!("/api/v0/tracks/{track_id}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(track["title"], fixture.track_title);
        assert_eq!(
            track["artistCredits"][0]["name"], "First Artist",
            "{extension}"
        );
        assert_eq!(
            track["artistCredits"][1]["name"], "Second Artist",
            "{extension}"
        );
        let audio = &track["originalAudioFile"];
        assert_eq!(audio["id"], fixture.audio_file.id.to_string());
        assert_eq!(audio["mediaType"], media_type, "{extension}");
        assert_eq!(audio["bitDepth"], bit_depth, "{extension}");
        assert_eq!(
            audio["sampleRate"],
            if extension == "opus" { 24000 } else { 32000 },
            "{extension}"
        );
        assert_eq!(audio["channels"], 1);
        assert!(audio["bitrate"].as_u64().unwrap() > 0);
        assert!(track["durationMs"].as_u64().unwrap() >= 900, "{extension}");
        assert!(
            audio["playbackUrl"]
                .as_str()
                .unwrap()
                .ends_with(&format!(".{}", extension.to_uppercase()))
        );

        if extension == "flac" {
            // FLAC permits an unknown total sample count in STREAMINFO.
            let mut bytes = std::fs::read(&path).unwrap();
            assert_eq!(&bytes[..4], b"fLaC");
            assert_eq!(bytes[4] & 0x7f, 0);
            bytes[21] &= 0xf0;
            bytes[22..26].fill(0);
            std::fs::write(&path, bytes).unwrap();
            let storage = FileSystemMusicStorage::new(
                root.path().to_owned(),
                "https://media.example.com/".parse().unwrap(),
            )
            .unwrap();
            let releases = BuildMetadataIndexUseCase::new(Arc::new(storage))
                .execute()
                .unwrap();
            assert_eq!(releases.len(), 1);
            let app = router(Arc::new(InMemoryMetadataIndex::new(releases)));
            let (status, unknown) =
                request(app.clone(), &format!("/api/v0/tracks/{track_id}")).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(unknown["id"], track["id"]);
            assert_eq!(unknown["originalAudioFile"]["id"], audio["id"]);
            assert_eq!(unknown["durationMs"], Value::Null);
            assert_eq!(unknown["originalAudioFile"]["bitrate"], Value::Null);
            let (_, release) = request(app, &format!("/api/v0/releases/{release_id}")).await;
            assert_eq!(release["tracks"][0]["durationMs"], Value::Null);
        }

        // A corrupt newly supported file invalidates its whole release rather than disappearing from the scan.
        std::fs::write(
            directory.join(format!("229fc612-8a56-45d0-bf61-55b36ff3dcba.{extension}")),
            b"corrupt",
        )
        .unwrap();
        let storage = FileSystemMusicStorage::new(
            root.path().to_owned(),
            "https://media.example.com/".parse().unwrap(),
        )
        .unwrap();
        let scanned = crate::port::MusicStorage::scan_releases(&storage).unwrap();
        assert_eq!(scanned.len(), 1);
        assert!(scanned[0].is_err(), "{extension}");
    }
}
