//! TTML 解析和生成器内部使用的常量定义

pub mod tags {
    pub const TT: &str = "tt";
    pub const HEAD: &str = "head";
    pub const BODY: &str = "body";
    pub const DIV: &str = "div";
    pub const P: &str = "p";
    pub const SPAN: &str = "span";
    pub const METADATA: &str = "metadata";

    pub const TRANSLATIONS: &str = "translations";
    pub const TRANSLITERATIONS: &str = "transliterations";
    pub const TRANSLATION: &str = "translation";
    pub const TRANSLITERATION: &str = "transliteration";

    pub const TTM_AGENT: &str = "ttm:agent";
    pub const TTM_NAME: &str = "ttm:name";
    pub const TTM_TITLE: &str = "ttm:title";
    pub const AMLL_META: &str = "amll:meta";

    pub const SONGWRITERS: &str = "songwriters";
    pub const SONGWRITER: &str = "songwriter";
    pub const TEXT: &str = "text";
    pub const ITUNES_METADATA: &str = "iTunesMetadata";
}

pub mod attrs {
    pub const XMLNS: &str = "xmlns";
    pub const XMLNS_TTM: &str = "xmlns:ttm";
    pub const XMLNS_TTS: &str = "xmlns:tts";
    pub const XMLNS_ITUNES: &str = "xmlns:itunes";
    pub const XMLNS_AMLL: &str = "xmlns:amll";

    pub const XML_LANG: &str = "xml:lang";
    pub const XML_ID: &str = "xml:id";
    pub const ITUNES_TIMING: &str = "itunes:timing";
    pub const ITUNES_SONGPART: &str = "itunes:songPart";
    pub const ITUNES_SONGPART_KEBAB: &str = "itunes:song-part";
    pub const ITUNES_KEY: &str = "itunes:key";

    pub const TTM_AGENT: &str = "ttm:agent";
    pub const TTM_ROLE: &str = "ttm:role";
    pub const TTS_RUBY: &str = "tts:ruby";

    pub const AMLL_OBSCENE: &str = "amll:obscene";
    pub const AMLL_EMPTY_BEAT: &str = "amll:empty-beat";

    pub const BEGIN: &str = "begin";
    pub const END: &str = "end";
    pub const DUR: &str = "dur";
    pub const TYPE: &str = "type";
    pub const FOR: &str = "for";
    pub const KEY: &str = "key";
    pub const VALUE: &str = "value";
}

pub mod vals {
    pub const ROLE_BG: &str = "x-bg";
    pub const ROLE_TRANS: &str = "x-translation";
    pub const ROLE_ROM: &str = "x-roman";

    pub const RUBY_CONTAINER: &str = "container";
    pub const RUBY_BASE: &str = "base";
    pub const RUBY_TEXT_CONTAINER: &str = "textContainer";
    pub const RUBY_TEXT: &str = "text";

    pub const TRUE_STR: &str = "true";

    pub const NS_TTML: &str = "http://www.w3.org/ns/ttml";
    pub const NS_TTM: &str = "http://www.w3.org/ns/ttml#metadata";
    pub const NS_TTS: &str = "http://www.w3.org/ns/ttml#styling";
    pub const NS_ITUNES: &str = "http://music.apple.com/lyric-ttml-internal";
    pub const NS_AMLL: &str = "http://www.example.com/ns/amll";

    pub const META_MUSIC_NAME: &str = "musicName";
    pub const META_ARTISTS: &str = "artists";
    pub const META_ALBUM: &str = "album";
    pub const META_ISRC: &str = "isrc";
    pub const META_GITHUB_ID: &str = "ttmlAuthorGithub";
    pub const META_GITHUB_USER_NAME: &str = "ttmlAuthorGithubLogin";
    pub const META_NCM_ID: &str = "ncmMusicId";
    pub const META_QQ_ID: &str = "qqMusicId";
    pub const META_SPOTIFY_ID: &str = "spotifyId";
    pub const META_APPLE_ID: &str = "appleMusicId";
}
