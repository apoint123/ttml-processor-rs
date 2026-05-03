//! TTML 解析和生成器内部使用的常量定义

macro_rules! define_const_strs {
    ($($name:ident = $value:literal;)*) => {
        $(
            pub const $name: &str = $value;
        )*
    };
}

pub mod tags {
    define_const_strs! {
        TT = "tt";
        HEAD = "head";
        BODY = "body";
        DIV = "div";
        P = "p";
        SPAN = "span";
        METADATA = "metadata";

        TRANSLATIONS = "translations";
        TRANSLITERATIONS = "transliterations";
        TRANSLATION = "translation";
        TRANSLITERATION = "transliteration";

        TTM_AGENT = "ttm:agent";
        TTM_NAME = "ttm:name";
        TTM_TITLE = "ttm:title";
        AMLL_META = "amll:meta";

        SONGWRITERS = "songwriters";
        SONGWRITER = "songwriter";
        TEXT = "text";
        ITUNES_METADATA = "iTunesMetadata";
    }
}

pub mod attrs {
    define_const_strs! {
        XMLNS = "xmlns";
        XMLNS_TTM = "xmlns:ttm";
        XMLNS_TTS = "xmlns:tts";
        XMLNS_ITUNES = "xmlns:itunes";
        XMLNS_AMLL = "xmlns:amll";

        XML_LANG = "xml:lang";
        XML_ID = "xml:id";
        ITUNES_TIMING = "itunes:timing";
        ITUNES_SONGPART = "itunes:songPart";
        ITUNES_SONGPART_KEBAB = "itunes:song-part";
        ITUNES_KEY = "itunes:key";

        TTM_AGENT = "ttm:agent";
        TTM_ROLE = "ttm:role";
        TTS_RUBY = "tts:ruby";

        AMLL_OBSCENE = "amll:obscene";
        AMLL_EMPTY_BEAT = "amll:empty-beat";

        BEGIN = "begin";
        END = "end";
        DUR = "dur";
        TYPE = "type";
        FOR = "for";
        KEY = "key";
        VALUE = "value";
    }
}

pub mod vals {
    define_const_strs! {
        ROLE_BG = "x-bg";
        ROLE_TRANS = "x-translation";
        ROLE_ROM = "x-roman";

        RUBY_CONTAINER = "container";
        RUBY_BASE = "base";
        RUBY_TEXT_CONTAINER = "textContainer";
        RUBY_TEXT = "text";

        TRUE_STR = "true";

        NS_TTML = "http://www.w3.org/ns/ttml";
        NS_TTM = "http://www.w3.org/ns/ttml#metadata";
        NS_TTS = "http://www.w3.org/ns/ttml#styling";
        NS_ITUNES = "http://music.apple.com/lyric-ttml-internal";
        NS_AMLL = "http://www.example.com/ns/amll";

        META_MUSIC_NAME = "musicName";
        META_ARTISTS = "artists";
        META_ALBUM = "album";
        META_ISRC = "isrc";
        META_GITHUB_ID = "ttmlAuthorGithub";
        META_GITHUB_USER_NAME = "ttmlAuthorGithubLogin";
        META_NCM_ID = "ncmMusicId";
        META_QQ_ID = "qqMusicId";
        META_SPOTIFY_ID = "spotifyId";
        META_APPLE_ID = "appleMusicId";
    }
}
