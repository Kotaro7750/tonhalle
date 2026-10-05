# ドキュメントのスコープ
本ドキュメントは以下をスコープとする。

- ドメインモデル

なお、実際の音源データファイルに関する事項については本ドキュメントのスコープ外であり、[mudic_data](./music_data.md)にて定義する。

# ドメインモデル
## 各エンティティの定義

- Release: アルバムやシングルといったリリース単位のメタデータを保持する
- Track: Release内における個々の楽曲単位のメタデータを保持する
- AudioFile: Trackに紐づく実際の音源データに関するメタデータを保持する
- Artist: アルバムや楽曲に関わるアーティストのメタデータを保持する

## ER図

```mermaid
erDiagram
    Release ||--|{ Track : "contains"
    Track ||--|{ AudioFile : "has"


    Artist ||--o{ TrackArtistCredit : "credited as"
    Track ||--|{ TrackArtistCredit : "has"
    Release ||--|{ ReleaseArtistCredit : "has"
    Artist ||--o{ ReleaseArtistCredit : "credited as"

    Release {
        uuid id PK
        string title
        date release_date
    }

    Track {
        uuid id PK
        uuid album_id FK
        int disc_number
        int track_number
        string title
        int duration_ms
    }

    AudioFile {
        uuid id PK
        uuid track_id FK
        string object_key
        string format
        int sample_rate
        int bit_depth
        int channels
        int bitrate
    }

    Artist {
        uuid id PK
        string name
    }

    ReleaseArtistCredit {
        uuid album_id FK
        uuid artist_id FK
        int position
        string join_phrase
    }

    TrackArtistCredit {
        uuid track_id FK
        uuid artist_id FK
        int position
        string join_phrase
    }
```
