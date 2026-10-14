# ドキュメントのスコープ

このファイルは、サンプル音源の内容、出典、権利表示、加工内容を記録する。

# Git LFSによる取得

音源ファイル（WAV・FLAC・AAC・M4A・OGG・MP3）は[.gitattributes](../.gitattributes)でGit LFSの管理対象にしている。
リポジトリをcloneした後、リポジトリルートで以下を実行すると音源の実体を取得できる。

```sh
mise install git-lfs
mise exec -- git lfs install --local
mise exec -- git lfs pull
```

音源の追加・更新は通常の`git add`、`git commit`、`git push`で行える。
Git LFSのpre-pushフックが音源をアップロードする。
GitHubが生成するZIP・tarballにも音源の実体を含めたい場合は、リポジトリのSettingsで[Git LFSオブジェクトをアーカイブに含める設定](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/managing-git-lfs-objects-in-archives-of-your-repository)を有効にする。

# 音源一覧

バックエンドテストとローカル開発用に、3リリース・6トラックを配置している。
リリース名に含まれる`Tonhalle Sample`、リリース日`2026-10-13`、トラック番号は、このサンプル用に設定したものであり、原盤の発売情報ではない。

| リリース | アーティスト | トラック数 | 内容 |
| --- | --- | --- | --- |
| [Bach: Short Preludes (Tonhalle Sample Album)](./release-ea57599b1f0c33d1d943ff20226ad7d418d79a1436217827aa09812607eaad64/) | Kimiko Ishizaka | 4 | 平均律クラヴィーア曲集第1巻の前奏曲第15番・第23番（各55秒・60秒）。第15番はWAV・AAC・M4Aの形式別サンプルとして収録 |
| [Bach: Prelude No. 11 (Tonhalle Sample Single)](./release-21fb4363f6c25b9a69da619ace8e12bcb27c9b1047a68de87a3433ed43321ac2/) | Kimiko Ishizaka | 1 | 同曲集の前奏曲第11番を収めた1曲リリース（64秒） |
| [Vieuxtemps: Caprice 'Yankee Doodle' (Tonhalle Sample Collaboration)](./release-1ac1508956628bfc1927b6fde571efbd604820dbacdeb82ca5d5d58eafe76be7/) | Elias Breeskin、Rudolph Gruen | 1 | ヴァイオリンとピアノの共演を収めた1曲リリース |

# 出典と権利表示

音源はWikimedia Commonsから取得した。
2026-10-13に各音源のファイルページで、楽曲だけでなく録音の権利表示を確認した。
音源には、リポジトリルートのMIT Licenseではなく、以下の出典ごとの権利表示が適用される。

| トラック | 録音の権利表示 | 出典 |
| --- | --- | --- |
| Prelude No. 15 in G major, BWV 860 | [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Kimiko_Ishizaka_-_Bach_-_Well-Tempered_Clavier,_Book_1_-_29_Prelude_No._15_in_G_major,_BWV_860.ogg) |
| Prelude No. 23 in B major, BWV 868 | [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Kimiko_Ishizaka_-_Bach_-_Well-Tempered_Clavier,_Book_1_-_45_Prelude_No._23_in_B_major,_BWV_868.ogg) |
| Prelude No. 11 in F major, BWV 856 | [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Kimiko_Ishizaka_-_Bach_-_Well-Tempered_Clavier,_Book_1_-_21_Prelude_No._11_in_F_major,_BWV_856.ogg) |
| Caprice 'Yankee Doodle' | 保護期間満了によるパブリックドメイン。[Public Domain Mark 1.0](https://creativecommons.org/publicdomain/mark/1.0/)の表示あり | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:CC0-CH_-_Elias_Breeskin_and_Rudolph_Gruen_-_Caprice_-_Yankee_Doodle_-_Brunswick-13012b-3651.flac) |

バッハの3曲はJohann Sebastian Bach（1750年没）の作曲で、演奏者はKimiko Ishizakaである。
[Open Well-Tempered Clavier](https://welltemperedclavier.org/)の録音から短い前奏曲を選び、各曲の全区間を使用している。
元の曲集全体を収めたリリースではなく、サンプル用に選んだ曲をまとめたものである。

「Caprice 'Yankee Doodle'」はHenri Vieuxtemps（1881年没）の作品で、Elias Breeskinがヴァイオリン、Rudolph Gruenがピアノを演奏している。
出典では1920年2月にニューヨークで録音・初回発売された音源とされ、編曲者は記載されていない。
1923年より前に初回発売された録音の米国での保護期間については、[U.S. Copyright Office](https://www.copyright.gov/music-modernization/pre1972-soundrecordings/)が2021年末で終了したことを説明している。
デジタル化された音源はCC-ZERO-PROJECTとWikimedia CHの協力による提供である。
元ファイル名に`CC0-CH`を含むが、ファイルページの権利表示はパブリックドメインであり、CC0ライセンスとして扱ってはいない。

個別の出典ページの版、ダウンロードURL、原音源のハッシュ値、配置したファイルのハッシュ値は[sources.json](./sources.json)に記録している。

# メタデータと配置

リリースとトラックの情報は音源内のタグに格納している。WAV・AAC（ADTS）・MP3はID3v2.4、FLAC・OGG（Vorbis）はVorbis Comments、M4AはiTunes形式のilstを使用する。
以下はID3v2.4でのフィールド名であり、他形式にも対応する情報を保存している。

- `TALB`：サンプルのリリース名
- `TPE2`：リリースのアーティスト。共演音源は2つの独立した値
- `TPE1`：トラックのアーティスト。共演音源は2つの独立した値
- `TIT2`：トラック名
- `TDRC`：サンプル用のリリース日
- `TPOS`、`TRCK`：1から始まるディスク番号とトラック番号
- `TCOM`：作曲者
- `TCOP`、`WCOP`、`WOAS`、`COMM`：権利表示、出典、加工内容とクレジット

MusicBrainzの識別子は付与していない。
リリースディレクトリ名には[domain.md](../docs/domain.md)の規則でメタデータから計算したReleaseの識別子を使用し、音源ファイル名には発行済みのAudioFile用UUID v4を使用している。
[sources.json](./sources.json)は出典とサンプルの確認用の一覧であり、音楽メタデータの取得元は音源内のタグである。

# 音源の加工

取得時には音源の全区間を、22,050 Hz・16 bit・モノラルの非圧縮PCM WAV（`pcm_s16le`）に変換した。
2026-10-14に、そのWAVから以下の形式別サンプルを用意した。切り出しや再生速度の変更は行っていない。

| 形式 | 曲 | 加工 |
| --- | --- | --- |
| WAV | 前奏曲第15番 | 既存ファイルを維持 |
| FLAC | 前奏曲第23番 | 可逆圧縮、compression level 8 |
| OGG（Vorbis） | 前奏曲第11番 | 非可逆圧縮、quality 3 |
| MP3 | Caprice 'Yankee Doodle' | 非可逆圧縮、64 kbit/s |
| AAC（ADTS） | 前奏曲第15番 | 非可逆圧縮、64 kbit/s |
| M4A（AAC） | 前奏曲第15番 | 非可逆圧縮、64 kbit/s |

形式別に追加した第15番には、独立したトラック番号とAudioFile UUIDを付与している。
短い曲を選び、チャンネル数とサンプルレートを下げることでリポジトリ内の容量を抑えており、現在の音源の合計容量は約5.7 MiBである（変更前は約15.3 MiB）。
GitにはLFSポインタを保存するが、削除・置換した音源の過去版はGit履歴から参照されている限りLFS側に残る。この変更だけで過去版を含む容量が減るわけではない。
元のOgg音源は非可逆圧縮されているため、WAVへの変換で音質が向上したり、失われた情報が復元されたりするものではない。
元音源にあった不明なアーティスト名やリリース名などのタグは、確認した演奏者とサンプル用のメタデータに置き換えている。
