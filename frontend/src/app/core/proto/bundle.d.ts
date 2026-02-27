import * as $protobuf from "protobufjs";
import Long = require("long");
/** Namespace petajournal. */
export namespace petajournal {

    /** Properties of a Timestamp. */
    interface ITimestamp {

        /** Timestamp millis */
        millis?: (number|Long|null);
    }

    /** Represents a Timestamp. */
    class Timestamp implements ITimestamp {

        /**
         * Constructs a new Timestamp.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ITimestamp);

        /** Timestamp millis. */
        public millis: (number|Long);

        /**
         * Creates a new Timestamp instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Timestamp instance
         */
        public static create(properties?: petajournal.ITimestamp): petajournal.Timestamp;

        /**
         * Encodes the specified Timestamp message. Does not implicitly {@link petajournal.Timestamp.verify|verify} messages.
         * @param message Timestamp message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ITimestamp, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Timestamp message, length delimited. Does not implicitly {@link petajournal.Timestamp.verify|verify} messages.
         * @param message Timestamp message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ITimestamp, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Timestamp message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Timestamp
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Timestamp;

        /**
         * Decodes a Timestamp message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Timestamp
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.Timestamp;

        /**
         * Verifies a Timestamp message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Timestamp message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Timestamp
         */
        public static fromObject(object: { [k: string]: any }): petajournal.Timestamp;

        /**
         * Creates a plain object from a Timestamp message. Also converts values to other types if specified.
         * @param message Timestamp
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.Timestamp, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Timestamp to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Timestamp
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Page. */
    interface IPage {

        /** Page id */
        id?: (string|null);

        /** Page dirPath */
        dirPath?: (string|null);

        /** Page name */
        name?: (string|null);

        /** Page type */
        type?: (string|null);

        /** Page createdDate */
        createdDate?: (petajournal.ITimestamp|null);

        /** Page modifiedDate */
        modifiedDate?: (petajournal.ITimestamp|null);

        /** Page tags */
        tags?: (string[]|null);
    }

    /** Represents a Page. */
    class Page implements IPage {

        /**
         * Constructs a new Page.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IPage);

        /** Page id. */
        public id: string;

        /** Page dirPath. */
        public dirPath: string;

        /** Page name. */
        public name: string;

        /** Page type. */
        public type: string;

        /** Page createdDate. */
        public createdDate?: (petajournal.ITimestamp|null);

        /** Page modifiedDate. */
        public modifiedDate?: (petajournal.ITimestamp|null);

        /** Page tags. */
        public tags: string[];

        /**
         * Creates a new Page instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Page instance
         */
        public static create(properties?: petajournal.IPage): petajournal.Page;

        /**
         * Encodes the specified Page message. Does not implicitly {@link petajournal.Page.verify|verify} messages.
         * @param message Page message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IPage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Page message, length delimited. Does not implicitly {@link petajournal.Page.verify|verify} messages.
         * @param message Page message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IPage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Page message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Page
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Page;

        /**
         * Decodes a Page message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Page
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.Page;

        /**
         * Verifies a Page message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Page message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Page
         */
        public static fromObject(object: { [k: string]: any }): petajournal.Page;

        /**
         * Creates a plain object from a Page message. Also converts values to other types if specified.
         * @param message Page
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.Page, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Page to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Page
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Block. */
    interface IBlock {

        /** Block id */
        id?: (string|null);

        /** Block pageId */
        pageId?: (string|null);

        /** Block dirPath */
        dirPath?: (string|null);

        /** Block name */
        name?: (string|null);

        /** Block type */
        type?: (string|null);

        /** Block createdDate */
        createdDate?: (petajournal.ITimestamp|null);

        /** Block modifiedDate */
        modifiedDate?: (petajournal.ITimestamp|null);

        /** Block idx */
        idx?: (string|null);

        /** Block level */
        level?: (number|null);

        /** Block body */
        body?: (string|null);

        /** Block html */
        html?: (string|null);

        /** Block collapsed */
        collapsed?: (boolean|null);

        /** Block status */
        status?: (string|null);

        /** Block types */
        types?: (string[]|null);
    }

    /** Represents a Block. */
    class Block implements IBlock {

        /**
         * Constructs a new Block.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IBlock);

        /** Block id. */
        public id: string;

        /** Block pageId. */
        public pageId: string;

        /** Block dirPath. */
        public dirPath: string;

        /** Block name. */
        public name: string;

        /** Block type. */
        public type: string;

        /** Block createdDate. */
        public createdDate?: (petajournal.ITimestamp|null);

        /** Block modifiedDate. */
        public modifiedDate?: (petajournal.ITimestamp|null);

        /** Block idx. */
        public idx: string;

        /** Block level. */
        public level: number;

        /** Block body. */
        public body: string;

        /** Block html. */
        public html: string;

        /** Block collapsed. */
        public collapsed: boolean;

        /** Block status. */
        public status: string;

        /** Block types. */
        public types: string[];

        /**
         * Creates a new Block instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Block instance
         */
        public static create(properties?: petajournal.IBlock): petajournal.Block;

        /**
         * Encodes the specified Block message. Does not implicitly {@link petajournal.Block.verify|verify} messages.
         * @param message Block message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IBlock, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Block message, length delimited. Does not implicitly {@link petajournal.Block.verify|verify} messages.
         * @param message Block message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IBlock, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Block message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Block
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Block;

        /**
         * Decodes a Block message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Block
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.Block;

        /**
         * Verifies a Block message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Block message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Block
         */
        public static fromObject(object: { [k: string]: any }): petajournal.Block;

        /**
         * Creates a plain object from a Block message. Also converts values to other types if specified.
         * @param message Block
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.Block, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Block to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Block
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchRequest. */
    interface ISearchRequest {

        /** SearchRequest query */
        query?: (string|null);

        /** SearchRequest maxResults */
        maxResults?: (number|null);
    }

    /** Represents a SearchRequest. */
    class SearchRequest implements ISearchRequest {

        /**
         * Constructs a new SearchRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchRequest);

        /** SearchRequest query. */
        public query: string;

        /** SearchRequest maxResults. */
        public maxResults: number;

        /**
         * Creates a new SearchRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchRequest instance
         */
        public static create(properties?: petajournal.ISearchRequest): petajournal.SearchRequest;

        /**
         * Encodes the specified SearchRequest message. Does not implicitly {@link petajournal.SearchRequest.verify|verify} messages.
         * @param message SearchRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchRequest message, length delimited. Does not implicitly {@link petajournal.SearchRequest.verify|verify} messages.
         * @param message SearchRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchRequest;

        /**
         * Decodes a SearchRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchRequest;

        /**
         * Verifies a SearchRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchRequest;

        /**
         * Creates a plain object from a SearchRequest message. Also converts values to other types if specified.
         * @param message SearchRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchResponse. */
    interface ISearchResponse {

        /** SearchResponse results */
        results?: (petajournal.ISearchResult[]|null);

        /** SearchResponse totalHits */
        totalHits?: (number|null);
    }

    /** Represents a SearchResponse. */
    class SearchResponse implements ISearchResponse {

        /**
         * Constructs a new SearchResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchResponse);

        /** SearchResponse results. */
        public results: petajournal.ISearchResult[];

        /** SearchResponse totalHits. */
        public totalHits: number;

        /**
         * Creates a new SearchResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchResponse instance
         */
        public static create(properties?: petajournal.ISearchResponse): petajournal.SearchResponse;

        /**
         * Encodes the specified SearchResponse message. Does not implicitly {@link petajournal.SearchResponse.verify|verify} messages.
         * @param message SearchResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchResponse message, length delimited. Does not implicitly {@link petajournal.SearchResponse.verify|verify} messages.
         * @param message SearchResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchResponse;

        /**
         * Decodes a SearchResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchResponse;

        /**
         * Verifies a SearchResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchResponse;

        /**
         * Creates a plain object from a SearchResponse message. Also converts values to other types if specified.
         * @param message SearchResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchResult. */
    interface ISearchResult {

        /** SearchResult pageId */
        pageId?: (string|null);

        /** SearchResult pageTitle */
        pageTitle?: (string|null);

        /** SearchResult blockId */
        blockId?: (string|null);

        /** SearchResult snippet */
        snippet?: (string|null);

        /** SearchResult score */
        score?: (number|null);
    }

    /** Represents a SearchResult. */
    class SearchResult implements ISearchResult {

        /**
         * Constructs a new SearchResult.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchResult);

        /** SearchResult pageId. */
        public pageId: string;

        /** SearchResult pageTitle. */
        public pageTitle: string;

        /** SearchResult blockId. */
        public blockId: string;

        /** SearchResult snippet. */
        public snippet: string;

        /** SearchResult score. */
        public score: number;

        /**
         * Creates a new SearchResult instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchResult instance
         */
        public static create(properties?: petajournal.ISearchResult): petajournal.SearchResult;

        /**
         * Encodes the specified SearchResult message. Does not implicitly {@link petajournal.SearchResult.verify|verify} messages.
         * @param message SearchResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchResult message, length delimited. Does not implicitly {@link petajournal.SearchResult.verify|verify} messages.
         * @param message SearchResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchResult message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchResult;

        /**
         * Decodes a SearchResult message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchResult;

        /**
         * Verifies a SearchResult message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchResult message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchResult
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchResult;

        /**
         * Creates a plain object from a SearchResult message. Also converts values to other types if specified.
         * @param message SearchResult
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchResult, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchResult to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchResult
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a BookmarkGroup. */
    interface IBookmarkGroup {

        /** BookmarkGroup id */
        id?: (string|null);

        /** BookmarkGroup name */
        name?: (string|null);

        /** BookmarkGroup sortOrder */
        sortOrder?: (string|null);

        /** BookmarkGroup createdAt */
        createdAt?: (petajournal.ITimestamp|null);
    }

    /** Represents a BookmarkGroup. */
    class BookmarkGroup implements IBookmarkGroup {

        /**
         * Constructs a new BookmarkGroup.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IBookmarkGroup);

        /** BookmarkGroup id. */
        public id: string;

        /** BookmarkGroup name. */
        public name: string;

        /** BookmarkGroup sortOrder. */
        public sortOrder: string;

        /** BookmarkGroup createdAt. */
        public createdAt?: (petajournal.ITimestamp|null);

        /**
         * Creates a new BookmarkGroup instance using the specified properties.
         * @param [properties] Properties to set
         * @returns BookmarkGroup instance
         */
        public static create(properties?: petajournal.IBookmarkGroup): petajournal.BookmarkGroup;

        /**
         * Encodes the specified BookmarkGroup message. Does not implicitly {@link petajournal.BookmarkGroup.verify|verify} messages.
         * @param message BookmarkGroup message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IBookmarkGroup, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified BookmarkGroup message, length delimited. Does not implicitly {@link petajournal.BookmarkGroup.verify|verify} messages.
         * @param message BookmarkGroup message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IBookmarkGroup, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a BookmarkGroup message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns BookmarkGroup
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.BookmarkGroup;

        /**
         * Decodes a BookmarkGroup message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns BookmarkGroup
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.BookmarkGroup;

        /**
         * Verifies a BookmarkGroup message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a BookmarkGroup message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns BookmarkGroup
         */
        public static fromObject(object: { [k: string]: any }): petajournal.BookmarkGroup;

        /**
         * Creates a plain object from a BookmarkGroup message. Also converts values to other types if specified.
         * @param message BookmarkGroup
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.BookmarkGroup, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this BookmarkGroup to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for BookmarkGroup
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Bookmark. */
    interface IBookmark {

        /** Bookmark id */
        id?: (string|null);

        /** Bookmark groupId */
        groupId?: (string|null);

        /** Bookmark targetType */
        targetType?: (string|null);

        /** Bookmark targetId */
        targetId?: (string|null);

        /** Bookmark sortOrder */
        sortOrder?: (string|null);

        /** Bookmark createdAt */
        createdAt?: (petajournal.ITimestamp|null);

        /** Bookmark targetPage */
        targetPage?: (petajournal.IPage|null);
    }

    /** Represents a Bookmark. */
    class Bookmark implements IBookmark {

        /**
         * Constructs a new Bookmark.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IBookmark);

        /** Bookmark id. */
        public id: string;

        /** Bookmark groupId. */
        public groupId: string;

        /** Bookmark targetType. */
        public targetType: string;

        /** Bookmark targetId. */
        public targetId: string;

        /** Bookmark sortOrder. */
        public sortOrder: string;

        /** Bookmark createdAt. */
        public createdAt?: (petajournal.ITimestamp|null);

        /** Bookmark targetPage. */
        public targetPage?: (petajournal.IPage|null);

        /**
         * Creates a new Bookmark instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Bookmark instance
         */
        public static create(properties?: petajournal.IBookmark): petajournal.Bookmark;

        /**
         * Encodes the specified Bookmark message. Does not implicitly {@link petajournal.Bookmark.verify|verify} messages.
         * @param message Bookmark message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IBookmark, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Bookmark message, length delimited. Does not implicitly {@link petajournal.Bookmark.verify|verify} messages.
         * @param message Bookmark message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IBookmark, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Bookmark message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Bookmark
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Bookmark;

        /**
         * Decodes a Bookmark message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Bookmark
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.Bookmark;

        /**
         * Verifies a Bookmark message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Bookmark message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Bookmark
         */
        public static fromObject(object: { [k: string]: any }): petajournal.Bookmark;

        /**
         * Creates a plain object from a Bookmark message. Also converts values to other types if specified.
         * @param message Bookmark
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.Bookmark, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Bookmark to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Bookmark
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ApiRequest. */
    interface IApiRequest {

        /** ApiRequest requestId */
        requestId?: (string|null);

        /** ApiRequest getPageById */
        getPageById?: (petajournal.IGetPageByIdRequest|null);

        /** ApiRequest getOrCreateDoc */
        getOrCreateDoc?: (petajournal.IGetOrCreateDocRequest|null);

        /** ApiRequest deletePage */
        deletePage?: (petajournal.IDeletePageRequest|null);

        /** ApiRequest updatePage */
        updatePage?: (petajournal.IUpdatePageRequest|null);

        /** ApiRequest createBlock */
        createBlock?: (petajournal.ICreateBlockRequest|null);

        /** ApiRequest updateBlock */
        updateBlock?: (petajournal.IUpdateBlockRequest|null);

        /** ApiRequest moveBlocks */
        moveBlocks?: (petajournal.IMoveBlocksRequest|null);

        /** ApiRequest deleteBlock */
        deleteBlock?: (petajournal.IDeleteBlockRequest|null);

        /** ApiRequest updateBlockStatus */
        updateBlockStatus?: (petajournal.IUpdateBlockStatusRequest|null);

        /** ApiRequest searchPageNames */
        searchPageNames?: (petajournal.ISearchPageNamesRequest|null);

        /** ApiRequest searchFullText */
        searchFullText?: (petajournal.ISearchFullTextRequest|null);

        /** ApiRequest getRecentPages */
        getRecentPages?: (petajournal.IGetRecentPagesRequest|null);

        /** ApiRequest getBacklinks */
        getBacklinks?: (petajournal.IGetBacklinksRequest|null);

        /** ApiRequest getMostRecentJournalBefore */
        getMostRecentJournalBefore?: (petajournal.IGetMostRecentJournalBeforeRequest|null);

        /** ApiRequest getBlocksByStatus */
        getBlocksByStatus?: (petajournal.IGetBlocksByStatusRequest|null);

        /** ApiRequest getConfig */
        getConfig?: (petajournal.IGetConfigRequest|null);

        /** ApiRequest updateConfig */
        updateConfig?: (petajournal.IUpdateConfigRequest|null);

        /** ApiRequest getOrCreateJournal */
        getOrCreateJournal?: (petajournal.IGetOrCreateJournalRequest|null);

        /** ApiRequest exportTreeMarkdown */
        exportTreeMarkdown?: (petajournal.IExportTreeMarkdownRequest|null);

        /** ApiRequest importTreeMarkdown */
        importTreeMarkdown?: (petajournal.IImportTreeMarkdownRequest|null);

        /** ApiRequest getBookmarks */
        getBookmarks?: (petajournal.IGetBookmarksRequest|null);

        /** ApiRequest createBookmarkGroup */
        createBookmarkGroup?: (petajournal.ICreateBookmarkGroupRequest|null);

        /** ApiRequest createBookmark */
        createBookmark?: (petajournal.ICreateBookmarkRequest|null);

        /** ApiRequest deleteBookmark */
        deleteBookmark?: (petajournal.IDeleteBookmarkRequest|null);

        /** ApiRequest deleteBookmarkGroup */
        deleteBookmarkGroup?: (petajournal.IDeleteBookmarkGroupRequest|null);

        /** ApiRequest moveBookmark */
        moveBookmark?: (petajournal.IMoveBookmarkRequest|null);

        /** ApiRequest moveBookmarkGroup */
        moveBookmarkGroup?: (petajournal.IMoveBookmarkGroupRequest|null);

        /** ApiRequest updateBookmarkGroup */
        updateBookmarkGroup?: (petajournal.IUpdateBookmarkGroupRequest|null);
    }

    /** Represents an ApiRequest. */
    class ApiRequest implements IApiRequest {

        /**
         * Constructs a new ApiRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IApiRequest);

        /** ApiRequest requestId. */
        public requestId: string;

        /** ApiRequest getPageById. */
        public getPageById?: (petajournal.IGetPageByIdRequest|null);

        /** ApiRequest getOrCreateDoc. */
        public getOrCreateDoc?: (petajournal.IGetOrCreateDocRequest|null);

        /** ApiRequest deletePage. */
        public deletePage?: (petajournal.IDeletePageRequest|null);

        /** ApiRequest updatePage. */
        public updatePage?: (petajournal.IUpdatePageRequest|null);

        /** ApiRequest createBlock. */
        public createBlock?: (petajournal.ICreateBlockRequest|null);

        /** ApiRequest updateBlock. */
        public updateBlock?: (petajournal.IUpdateBlockRequest|null);

        /** ApiRequest moveBlocks. */
        public moveBlocks?: (petajournal.IMoveBlocksRequest|null);

        /** ApiRequest deleteBlock. */
        public deleteBlock?: (petajournal.IDeleteBlockRequest|null);

        /** ApiRequest updateBlockStatus. */
        public updateBlockStatus?: (petajournal.IUpdateBlockStatusRequest|null);

        /** ApiRequest searchPageNames. */
        public searchPageNames?: (petajournal.ISearchPageNamesRequest|null);

        /** ApiRequest searchFullText. */
        public searchFullText?: (petajournal.ISearchFullTextRequest|null);

        /** ApiRequest getRecentPages. */
        public getRecentPages?: (petajournal.IGetRecentPagesRequest|null);

        /** ApiRequest getBacklinks. */
        public getBacklinks?: (petajournal.IGetBacklinksRequest|null);

        /** ApiRequest getMostRecentJournalBefore. */
        public getMostRecentJournalBefore?: (petajournal.IGetMostRecentJournalBeforeRequest|null);

        /** ApiRequest getBlocksByStatus. */
        public getBlocksByStatus?: (petajournal.IGetBlocksByStatusRequest|null);

        /** ApiRequest getConfig. */
        public getConfig?: (petajournal.IGetConfigRequest|null);

        /** ApiRequest updateConfig. */
        public updateConfig?: (petajournal.IUpdateConfigRequest|null);

        /** ApiRequest getOrCreateJournal. */
        public getOrCreateJournal?: (petajournal.IGetOrCreateJournalRequest|null);

        /** ApiRequest exportTreeMarkdown. */
        public exportTreeMarkdown?: (petajournal.IExportTreeMarkdownRequest|null);

        /** ApiRequest importTreeMarkdown. */
        public importTreeMarkdown?: (petajournal.IImportTreeMarkdownRequest|null);

        /** ApiRequest getBookmarks. */
        public getBookmarks?: (petajournal.IGetBookmarksRequest|null);

        /** ApiRequest createBookmarkGroup. */
        public createBookmarkGroup?: (petajournal.ICreateBookmarkGroupRequest|null);

        /** ApiRequest createBookmark. */
        public createBookmark?: (petajournal.ICreateBookmarkRequest|null);

        /** ApiRequest deleteBookmark. */
        public deleteBookmark?: (petajournal.IDeleteBookmarkRequest|null);

        /** ApiRequest deleteBookmarkGroup. */
        public deleteBookmarkGroup?: (petajournal.IDeleteBookmarkGroupRequest|null);

        /** ApiRequest moveBookmark. */
        public moveBookmark?: (petajournal.IMoveBookmarkRequest|null);

        /** ApiRequest moveBookmarkGroup. */
        public moveBookmarkGroup?: (petajournal.IMoveBookmarkGroupRequest|null);

        /** ApiRequest updateBookmarkGroup. */
        public updateBookmarkGroup?: (petajournal.IUpdateBookmarkGroupRequest|null);

        /** ApiRequest payload. */
        public payload?: ("getPageById"|"getOrCreateDoc"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getBlocksByStatus"|"getConfig"|"updateConfig"|"getOrCreateJournal"|"exportTreeMarkdown"|"importTreeMarkdown"|"getBookmarks"|"createBookmarkGroup"|"createBookmark"|"deleteBookmark"|"deleteBookmarkGroup"|"moveBookmark"|"moveBookmarkGroup"|"updateBookmarkGroup");

        /**
         * Creates a new ApiRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ApiRequest instance
         */
        public static create(properties?: petajournal.IApiRequest): petajournal.ApiRequest;

        /**
         * Encodes the specified ApiRequest message. Does not implicitly {@link petajournal.ApiRequest.verify|verify} messages.
         * @param message ApiRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IApiRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ApiRequest message, length delimited. Does not implicitly {@link petajournal.ApiRequest.verify|verify} messages.
         * @param message ApiRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IApiRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ApiRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ApiRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ApiRequest;

        /**
         * Decodes an ApiRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ApiRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ApiRequest;

        /**
         * Verifies an ApiRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ApiRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ApiRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ApiRequest;

        /**
         * Creates a plain object from an ApiRequest message. Also converts values to other types if specified.
         * @param message ApiRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ApiRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ApiRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ApiRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ApiResponse. */
    interface IApiResponse {

        /** ApiResponse requestId */
        requestId?: (string|null);

        /** ApiResponse success */
        success?: (boolean|null);

        /** ApiResponse errorMessage */
        errorMessage?: (string|null);

        /** ApiResponse getPageById */
        getPageById?: (petajournal.IGetPageByIdResponse|null);

        /** ApiResponse getOrCreateDoc */
        getOrCreateDoc?: (petajournal.IGetOrCreateDocResponse|null);

        /** ApiResponse deletePage */
        deletePage?: (petajournal.IDeletePageResponse|null);

        /** ApiResponse updatePage */
        updatePage?: (petajournal.IUpdatePageResponse|null);

        /** ApiResponse createBlock */
        createBlock?: (petajournal.ICreateBlockResponse|null);

        /** ApiResponse updateBlock */
        updateBlock?: (petajournal.IUpdateBlockResponse|null);

        /** ApiResponse moveBlocks */
        moveBlocks?: (petajournal.IMoveBlocksResponse|null);

        /** ApiResponse deleteBlock */
        deleteBlock?: (petajournal.IDeleteBlockResponse|null);

        /** ApiResponse updateBlockStatus */
        updateBlockStatus?: (petajournal.IUpdateBlockStatusResponse|null);

        /** ApiResponse searchPageNames */
        searchPageNames?: (petajournal.ISearchPageNamesResponse|null);

        /** ApiResponse searchFullText */
        searchFullText?: (petajournal.ISearchFullTextResponse|null);

        /** ApiResponse getRecentPages */
        getRecentPages?: (petajournal.IGetRecentPagesResponse|null);

        /** ApiResponse getBacklinks */
        getBacklinks?: (petajournal.IGetBacklinksResponse|null);

        /** ApiResponse getMostRecentJournalBefore */
        getMostRecentJournalBefore?: (petajournal.IGetMostRecentJournalBeforeResponse|null);

        /** ApiResponse getOrCreateJournal */
        getOrCreateJournal?: (petajournal.IGetOrCreateJournalResponse|null);

        /** ApiResponse getBlocksByStatus */
        getBlocksByStatus?: (petajournal.IGetBlocksByStatusResponse|null);

        /** ApiResponse getConfig */
        getConfig?: (petajournal.IGetConfigResponse|null);

        /** ApiResponse updateConfig */
        updateConfig?: (petajournal.IUpdateConfigResponse|null);

        /** ApiResponse exportTreeMarkdown */
        exportTreeMarkdown?: (petajournal.IExportTreeMarkdownResponse|null);

        /** ApiResponse importTreeMarkdown */
        importTreeMarkdown?: (petajournal.IImportTreeMarkdownResponse|null);

        /** ApiResponse getBookmarks */
        getBookmarks?: (petajournal.IGetBookmarksResponse|null);

        /** ApiResponse createBookmarkGroup */
        createBookmarkGroup?: (petajournal.ICreateBookmarkGroupResponse|null);

        /** ApiResponse createBookmark */
        createBookmark?: (petajournal.ICreateBookmarkResponse|null);

        /** ApiResponse deleteBookmark */
        deleteBookmark?: (petajournal.IDeleteBookmarkResponse|null);

        /** ApiResponse deleteBookmarkGroup */
        deleteBookmarkGroup?: (petajournal.IDeleteBookmarkGroupResponse|null);

        /** ApiResponse moveBookmark */
        moveBookmark?: (petajournal.IMoveBookmarkResponse|null);

        /** ApiResponse moveBookmarkGroup */
        moveBookmarkGroup?: (petajournal.IMoveBookmarkGroupResponse|null);

        /** ApiResponse updateBookmarkGroup */
        updateBookmarkGroup?: (petajournal.IUpdateBookmarkGroupResponse|null);

        /** ApiResponse serverEvent */
        serverEvent?: (petajournal.IServerEvent|null);
    }

    /** Represents an ApiResponse. */
    class ApiResponse implements IApiResponse {

        /**
         * Constructs a new ApiResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IApiResponse);

        /** ApiResponse requestId. */
        public requestId: string;

        /** ApiResponse success. */
        public success: boolean;

        /** ApiResponse errorMessage. */
        public errorMessage: string;

        /** ApiResponse getPageById. */
        public getPageById?: (petajournal.IGetPageByIdResponse|null);

        /** ApiResponse getOrCreateDoc. */
        public getOrCreateDoc?: (petajournal.IGetOrCreateDocResponse|null);

        /** ApiResponse deletePage. */
        public deletePage?: (petajournal.IDeletePageResponse|null);

        /** ApiResponse updatePage. */
        public updatePage?: (petajournal.IUpdatePageResponse|null);

        /** ApiResponse createBlock. */
        public createBlock?: (petajournal.ICreateBlockResponse|null);

        /** ApiResponse updateBlock. */
        public updateBlock?: (petajournal.IUpdateBlockResponse|null);

        /** ApiResponse moveBlocks. */
        public moveBlocks?: (petajournal.IMoveBlocksResponse|null);

        /** ApiResponse deleteBlock. */
        public deleteBlock?: (petajournal.IDeleteBlockResponse|null);

        /** ApiResponse updateBlockStatus. */
        public updateBlockStatus?: (petajournal.IUpdateBlockStatusResponse|null);

        /** ApiResponse searchPageNames. */
        public searchPageNames?: (petajournal.ISearchPageNamesResponse|null);

        /** ApiResponse searchFullText. */
        public searchFullText?: (petajournal.ISearchFullTextResponse|null);

        /** ApiResponse getRecentPages. */
        public getRecentPages?: (petajournal.IGetRecentPagesResponse|null);

        /** ApiResponse getBacklinks. */
        public getBacklinks?: (petajournal.IGetBacklinksResponse|null);

        /** ApiResponse getMostRecentJournalBefore. */
        public getMostRecentJournalBefore?: (petajournal.IGetMostRecentJournalBeforeResponse|null);

        /** ApiResponse getOrCreateJournal. */
        public getOrCreateJournal?: (petajournal.IGetOrCreateJournalResponse|null);

        /** ApiResponse getBlocksByStatus. */
        public getBlocksByStatus?: (petajournal.IGetBlocksByStatusResponse|null);

        /** ApiResponse getConfig. */
        public getConfig?: (petajournal.IGetConfigResponse|null);

        /** ApiResponse updateConfig. */
        public updateConfig?: (petajournal.IUpdateConfigResponse|null);

        /** ApiResponse exportTreeMarkdown. */
        public exportTreeMarkdown?: (petajournal.IExportTreeMarkdownResponse|null);

        /** ApiResponse importTreeMarkdown. */
        public importTreeMarkdown?: (petajournal.IImportTreeMarkdownResponse|null);

        /** ApiResponse getBookmarks. */
        public getBookmarks?: (petajournal.IGetBookmarksResponse|null);

        /** ApiResponse createBookmarkGroup. */
        public createBookmarkGroup?: (petajournal.ICreateBookmarkGroupResponse|null);

        /** ApiResponse createBookmark. */
        public createBookmark?: (petajournal.ICreateBookmarkResponse|null);

        /** ApiResponse deleteBookmark. */
        public deleteBookmark?: (petajournal.IDeleteBookmarkResponse|null);

        /** ApiResponse deleteBookmarkGroup. */
        public deleteBookmarkGroup?: (petajournal.IDeleteBookmarkGroupResponse|null);

        /** ApiResponse moveBookmark. */
        public moveBookmark?: (petajournal.IMoveBookmarkResponse|null);

        /** ApiResponse moveBookmarkGroup. */
        public moveBookmarkGroup?: (petajournal.IMoveBookmarkGroupResponse|null);

        /** ApiResponse updateBookmarkGroup. */
        public updateBookmarkGroup?: (petajournal.IUpdateBookmarkGroupResponse|null);

        /** ApiResponse serverEvent. */
        public serverEvent?: (petajournal.IServerEvent|null);

        /** ApiResponse payload. */
        public payload?: ("getPageById"|"getOrCreateDoc"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getOrCreateJournal"|"getBlocksByStatus"|"getConfig"|"updateConfig"|"exportTreeMarkdown"|"importTreeMarkdown"|"getBookmarks"|"createBookmarkGroup"|"createBookmark"|"deleteBookmark"|"deleteBookmarkGroup"|"moveBookmark"|"moveBookmarkGroup"|"updateBookmarkGroup"|"serverEvent");

        /**
         * Creates a new ApiResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ApiResponse instance
         */
        public static create(properties?: petajournal.IApiResponse): petajournal.ApiResponse;

        /**
         * Encodes the specified ApiResponse message. Does not implicitly {@link petajournal.ApiResponse.verify|verify} messages.
         * @param message ApiResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IApiResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ApiResponse message, length delimited. Does not implicitly {@link petajournal.ApiResponse.verify|verify} messages.
         * @param message ApiResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IApiResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ApiResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ApiResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ApiResponse;

        /**
         * Decodes an ApiResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ApiResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ApiResponse;

        /**
         * Verifies an ApiResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ApiResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ApiResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ApiResponse;

        /**
         * Creates a plain object from an ApiResponse message. Also converts values to other types if specified.
         * @param message ApiResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ApiResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ApiResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ApiResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetPageByIdRequest. */
    interface IGetPageByIdRequest {

        /** GetPageByIdRequest pageId */
        pageId?: (string|null);
    }

    /** Represents a GetPageByIdRequest. */
    class GetPageByIdRequest implements IGetPageByIdRequest {

        /**
         * Constructs a new GetPageByIdRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetPageByIdRequest);

        /** GetPageByIdRequest pageId. */
        public pageId: string;

        /**
         * Creates a new GetPageByIdRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetPageByIdRequest instance
         */
        public static create(properties?: petajournal.IGetPageByIdRequest): petajournal.GetPageByIdRequest;

        /**
         * Encodes the specified GetPageByIdRequest message. Does not implicitly {@link petajournal.GetPageByIdRequest.verify|verify} messages.
         * @param message GetPageByIdRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetPageByIdRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetPageByIdRequest message, length delimited. Does not implicitly {@link petajournal.GetPageByIdRequest.verify|verify} messages.
         * @param message GetPageByIdRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetPageByIdRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetPageByIdRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetPageByIdRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetPageByIdRequest;

        /**
         * Decodes a GetPageByIdRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetPageByIdRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetPageByIdRequest;

        /**
         * Verifies a GetPageByIdRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetPageByIdRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetPageByIdRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetPageByIdRequest;

        /**
         * Creates a plain object from a GetPageByIdRequest message. Also converts values to other types if specified.
         * @param message GetPageByIdRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetPageByIdRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetPageByIdRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetPageByIdRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetPageByIdResponse. */
    interface IGetPageByIdResponse {

        /** GetPageByIdResponse page */
        page?: (petajournal.IPage|null);

        /** GetPageByIdResponse blocks */
        blocks?: (petajournal.IBlock[]|null);
    }

    /** Represents a GetPageByIdResponse. */
    class GetPageByIdResponse implements IGetPageByIdResponse {

        /**
         * Constructs a new GetPageByIdResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetPageByIdResponse);

        /** GetPageByIdResponse page. */
        public page?: (petajournal.IPage|null);

        /** GetPageByIdResponse blocks. */
        public blocks: petajournal.IBlock[];

        /**
         * Creates a new GetPageByIdResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetPageByIdResponse instance
         */
        public static create(properties?: petajournal.IGetPageByIdResponse): petajournal.GetPageByIdResponse;

        /**
         * Encodes the specified GetPageByIdResponse message. Does not implicitly {@link petajournal.GetPageByIdResponse.verify|verify} messages.
         * @param message GetPageByIdResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetPageByIdResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetPageByIdResponse message, length delimited. Does not implicitly {@link petajournal.GetPageByIdResponse.verify|verify} messages.
         * @param message GetPageByIdResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetPageByIdResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetPageByIdResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetPageByIdResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetPageByIdResponse;

        /**
         * Decodes a GetPageByIdResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetPageByIdResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetPageByIdResponse;

        /**
         * Verifies a GetPageByIdResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetPageByIdResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetPageByIdResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetPageByIdResponse;

        /**
         * Creates a plain object from a GetPageByIdResponse message. Also converts values to other types if specified.
         * @param message GetPageByIdResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetPageByIdResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetPageByIdResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetPageByIdResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetOrCreateDocRequest. */
    interface IGetOrCreateDocRequest {

        /** GetOrCreateDocRequest title */
        title?: (string|null);

        /** GetOrCreateDocRequest dirPath */
        dirPath?: (string|null);

        /** GetOrCreateDocRequest tags */
        tags?: (string[]|null);
    }

    /** Represents a GetOrCreateDocRequest. */
    class GetOrCreateDocRequest implements IGetOrCreateDocRequest {

        /**
         * Constructs a new GetOrCreateDocRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetOrCreateDocRequest);

        /** GetOrCreateDocRequest title. */
        public title: string;

        /** GetOrCreateDocRequest dirPath. */
        public dirPath: string;

        /** GetOrCreateDocRequest tags. */
        public tags: string[];

        /**
         * Creates a new GetOrCreateDocRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetOrCreateDocRequest instance
         */
        public static create(properties?: petajournal.IGetOrCreateDocRequest): petajournal.GetOrCreateDocRequest;

        /**
         * Encodes the specified GetOrCreateDocRequest message. Does not implicitly {@link petajournal.GetOrCreateDocRequest.verify|verify} messages.
         * @param message GetOrCreateDocRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetOrCreateDocRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetOrCreateDocRequest message, length delimited. Does not implicitly {@link petajournal.GetOrCreateDocRequest.verify|verify} messages.
         * @param message GetOrCreateDocRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetOrCreateDocRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetOrCreateDocRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetOrCreateDocRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetOrCreateDocRequest;

        /**
         * Decodes a GetOrCreateDocRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetOrCreateDocRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetOrCreateDocRequest;

        /**
         * Verifies a GetOrCreateDocRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetOrCreateDocRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetOrCreateDocRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetOrCreateDocRequest;

        /**
         * Creates a plain object from a GetOrCreateDocRequest message. Also converts values to other types if specified.
         * @param message GetOrCreateDocRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetOrCreateDocRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetOrCreateDocRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetOrCreateDocRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetOrCreateDocResponse. */
    interface IGetOrCreateDocResponse {

        /** GetOrCreateDocResponse page */
        page?: (petajournal.IPage|null);

        /** GetOrCreateDocResponse blocks */
        blocks?: (petajournal.IBlock[]|null);

        /** GetOrCreateDocResponse created */
        created?: (boolean|null);
    }

    /** Represents a GetOrCreateDocResponse. */
    class GetOrCreateDocResponse implements IGetOrCreateDocResponse {

        /**
         * Constructs a new GetOrCreateDocResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetOrCreateDocResponse);

        /** GetOrCreateDocResponse page. */
        public page?: (petajournal.IPage|null);

        /** GetOrCreateDocResponse blocks. */
        public blocks: petajournal.IBlock[];

        /** GetOrCreateDocResponse created. */
        public created: boolean;

        /**
         * Creates a new GetOrCreateDocResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetOrCreateDocResponse instance
         */
        public static create(properties?: petajournal.IGetOrCreateDocResponse): petajournal.GetOrCreateDocResponse;

        /**
         * Encodes the specified GetOrCreateDocResponse message. Does not implicitly {@link petajournal.GetOrCreateDocResponse.verify|verify} messages.
         * @param message GetOrCreateDocResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetOrCreateDocResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetOrCreateDocResponse message, length delimited. Does not implicitly {@link petajournal.GetOrCreateDocResponse.verify|verify} messages.
         * @param message GetOrCreateDocResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetOrCreateDocResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetOrCreateDocResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetOrCreateDocResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetOrCreateDocResponse;

        /**
         * Decodes a GetOrCreateDocResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetOrCreateDocResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetOrCreateDocResponse;

        /**
         * Verifies a GetOrCreateDocResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetOrCreateDocResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetOrCreateDocResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetOrCreateDocResponse;

        /**
         * Creates a plain object from a GetOrCreateDocResponse message. Also converts values to other types if specified.
         * @param message GetOrCreateDocResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetOrCreateDocResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetOrCreateDocResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetOrCreateDocResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeletePageRequest. */
    interface IDeletePageRequest {

        /** DeletePageRequest pageId */
        pageId?: (string|null);
    }

    /** Represents a DeletePageRequest. */
    class DeletePageRequest implements IDeletePageRequest {

        /**
         * Constructs a new DeletePageRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeletePageRequest);

        /** DeletePageRequest pageId. */
        public pageId: string;

        /**
         * Creates a new DeletePageRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeletePageRequest instance
         */
        public static create(properties?: petajournal.IDeletePageRequest): petajournal.DeletePageRequest;

        /**
         * Encodes the specified DeletePageRequest message. Does not implicitly {@link petajournal.DeletePageRequest.verify|verify} messages.
         * @param message DeletePageRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeletePageRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeletePageRequest message, length delimited. Does not implicitly {@link petajournal.DeletePageRequest.verify|verify} messages.
         * @param message DeletePageRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeletePageRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeletePageRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeletePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeletePageRequest;

        /**
         * Decodes a DeletePageRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeletePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeletePageRequest;

        /**
         * Verifies a DeletePageRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeletePageRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeletePageRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeletePageRequest;

        /**
         * Creates a plain object from a DeletePageRequest message. Also converts values to other types if specified.
         * @param message DeletePageRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeletePageRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeletePageRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeletePageRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeletePageResponse. */
    interface IDeletePageResponse {
    }

    /** Represents a DeletePageResponse. */
    class DeletePageResponse implements IDeletePageResponse {

        /**
         * Constructs a new DeletePageResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeletePageResponse);

        /**
         * Creates a new DeletePageResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeletePageResponse instance
         */
        public static create(properties?: petajournal.IDeletePageResponse): petajournal.DeletePageResponse;

        /**
         * Encodes the specified DeletePageResponse message. Does not implicitly {@link petajournal.DeletePageResponse.verify|verify} messages.
         * @param message DeletePageResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeletePageResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeletePageResponse message, length delimited. Does not implicitly {@link petajournal.DeletePageResponse.verify|verify} messages.
         * @param message DeletePageResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeletePageResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeletePageResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeletePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeletePageResponse;

        /**
         * Decodes a DeletePageResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeletePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeletePageResponse;

        /**
         * Verifies a DeletePageResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeletePageResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeletePageResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeletePageResponse;

        /**
         * Creates a plain object from a DeletePageResponse message. Also converts values to other types if specified.
         * @param message DeletePageResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeletePageResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeletePageResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeletePageResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdatePageRequest. */
    interface IUpdatePageRequest {

        /** UpdatePageRequest pageId */
        pageId?: (string|null);

        /** UpdatePageRequest title */
        title?: (string|null);

        /** UpdatePageRequest tags */
        tags?: (string[]|null);

        /** UpdatePageRequest dirPath */
        dirPath?: (string|null);
    }

    /** Represents an UpdatePageRequest. */
    class UpdatePageRequest implements IUpdatePageRequest {

        /**
         * Constructs a new UpdatePageRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdatePageRequest);

        /** UpdatePageRequest pageId. */
        public pageId: string;

        /** UpdatePageRequest title. */
        public title?: (string|null);

        /** UpdatePageRequest tags. */
        public tags: string[];

        /** UpdatePageRequest dirPath. */
        public dirPath?: (string|null);

        /**
         * Creates a new UpdatePageRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdatePageRequest instance
         */
        public static create(properties?: petajournal.IUpdatePageRequest): petajournal.UpdatePageRequest;

        /**
         * Encodes the specified UpdatePageRequest message. Does not implicitly {@link petajournal.UpdatePageRequest.verify|verify} messages.
         * @param message UpdatePageRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdatePageRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdatePageRequest message, length delimited. Does not implicitly {@link petajournal.UpdatePageRequest.verify|verify} messages.
         * @param message UpdatePageRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdatePageRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdatePageRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdatePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdatePageRequest;

        /**
         * Decodes an UpdatePageRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdatePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdatePageRequest;

        /**
         * Verifies an UpdatePageRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdatePageRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdatePageRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdatePageRequest;

        /**
         * Creates a plain object from an UpdatePageRequest message. Also converts values to other types if specified.
         * @param message UpdatePageRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdatePageRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdatePageRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdatePageRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdatePageResponse. */
    interface IUpdatePageResponse {

        /** UpdatePageResponse page */
        page?: (petajournal.IPage|null);
    }

    /** Represents an UpdatePageResponse. */
    class UpdatePageResponse implements IUpdatePageResponse {

        /**
         * Constructs a new UpdatePageResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdatePageResponse);

        /** UpdatePageResponse page. */
        public page?: (petajournal.IPage|null);

        /**
         * Creates a new UpdatePageResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdatePageResponse instance
         */
        public static create(properties?: petajournal.IUpdatePageResponse): petajournal.UpdatePageResponse;

        /**
         * Encodes the specified UpdatePageResponse message. Does not implicitly {@link petajournal.UpdatePageResponse.verify|verify} messages.
         * @param message UpdatePageResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdatePageResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdatePageResponse message, length delimited. Does not implicitly {@link petajournal.UpdatePageResponse.verify|verify} messages.
         * @param message UpdatePageResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdatePageResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdatePageResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdatePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdatePageResponse;

        /**
         * Decodes an UpdatePageResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdatePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdatePageResponse;

        /**
         * Verifies an UpdatePageResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdatePageResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdatePageResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdatePageResponse;

        /**
         * Creates a plain object from an UpdatePageResponse message. Also converts values to other types if specified.
         * @param message UpdatePageResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdatePageResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdatePageResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdatePageResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreateBlockRequest. */
    interface ICreateBlockRequest {

        /** CreateBlockRequest pageId */
        pageId?: (string|null);

        /** CreateBlockRequest afterBlockId */
        afterBlockId?: (string|null);

        /** CreateBlockRequest level */
        level?: (number|null);

        /** CreateBlockRequest body */
        body?: (string|null);

        /** CreateBlockRequest id */
        id?: (string|null);
    }

    /** Represents a CreateBlockRequest. */
    class CreateBlockRequest implements ICreateBlockRequest {

        /**
         * Constructs a new CreateBlockRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreateBlockRequest);

        /** CreateBlockRequest pageId. */
        public pageId: string;

        /** CreateBlockRequest afterBlockId. */
        public afterBlockId: string;

        /** CreateBlockRequest level. */
        public level: number;

        /** CreateBlockRequest body. */
        public body: string;

        /** CreateBlockRequest id. */
        public id: string;

        /**
         * Creates a new CreateBlockRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreateBlockRequest instance
         */
        public static create(properties?: petajournal.ICreateBlockRequest): petajournal.CreateBlockRequest;

        /**
         * Encodes the specified CreateBlockRequest message. Does not implicitly {@link petajournal.CreateBlockRequest.verify|verify} messages.
         * @param message CreateBlockRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreateBlockRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CreateBlockRequest message, length delimited. Does not implicitly {@link petajournal.CreateBlockRequest.verify|verify} messages.
         * @param message CreateBlockRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ICreateBlockRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreateBlockRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBlockRequest;

        /**
         * Decodes a CreateBlockRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CreateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.CreateBlockRequest;

        /**
         * Verifies a CreateBlockRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CreateBlockRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CreateBlockRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.CreateBlockRequest;

        /**
         * Creates a plain object from a CreateBlockRequest message. Also converts values to other types if specified.
         * @param message CreateBlockRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.CreateBlockRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CreateBlockRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for CreateBlockRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreateBlockResponse. */
    interface ICreateBlockResponse {

        /** CreateBlockResponse block */
        block?: (petajournal.IBlock|null);
    }

    /** Represents a CreateBlockResponse. */
    class CreateBlockResponse implements ICreateBlockResponse {

        /**
         * Constructs a new CreateBlockResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreateBlockResponse);

        /** CreateBlockResponse block. */
        public block?: (petajournal.IBlock|null);

        /**
         * Creates a new CreateBlockResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreateBlockResponse instance
         */
        public static create(properties?: petajournal.ICreateBlockResponse): petajournal.CreateBlockResponse;

        /**
         * Encodes the specified CreateBlockResponse message. Does not implicitly {@link petajournal.CreateBlockResponse.verify|verify} messages.
         * @param message CreateBlockResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreateBlockResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CreateBlockResponse message, length delimited. Does not implicitly {@link petajournal.CreateBlockResponse.verify|verify} messages.
         * @param message CreateBlockResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ICreateBlockResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreateBlockResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBlockResponse;

        /**
         * Decodes a CreateBlockResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CreateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.CreateBlockResponse;

        /**
         * Verifies a CreateBlockResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CreateBlockResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CreateBlockResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.CreateBlockResponse;

        /**
         * Creates a plain object from a CreateBlockResponse message. Also converts values to other types if specified.
         * @param message CreateBlockResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.CreateBlockResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CreateBlockResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for CreateBlockResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateBlockRequest. */
    interface IUpdateBlockRequest {

        /** UpdateBlockRequest blockId */
        blockId?: (string|null);

        /** UpdateBlockRequest body */
        body?: (string|null);

        /** UpdateBlockRequest level */
        level?: (number|null);

        /** UpdateBlockRequest collapsed */
        collapsed?: (boolean|null);

        /** UpdateBlockRequest status */
        status?: (string|null);
    }

    /** Represents an UpdateBlockRequest. */
    class UpdateBlockRequest implements IUpdateBlockRequest {

        /**
         * Constructs a new UpdateBlockRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateBlockRequest);

        /** UpdateBlockRequest blockId. */
        public blockId: string;

        /** UpdateBlockRequest body. */
        public body?: (string|null);

        /** UpdateBlockRequest level. */
        public level?: (number|null);

        /** UpdateBlockRequest collapsed. */
        public collapsed?: (boolean|null);

        /** UpdateBlockRequest status. */
        public status?: (string|null);

        /**
         * Creates a new UpdateBlockRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateBlockRequest instance
         */
        public static create(properties?: petajournal.IUpdateBlockRequest): petajournal.UpdateBlockRequest;

        /**
         * Encodes the specified UpdateBlockRequest message. Does not implicitly {@link petajournal.UpdateBlockRequest.verify|verify} messages.
         * @param message UpdateBlockRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateBlockRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateBlockRequest message, length delimited. Does not implicitly {@link petajournal.UpdateBlockRequest.verify|verify} messages.
         * @param message UpdateBlockRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateBlockRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateBlockRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockRequest;

        /**
         * Decodes an UpdateBlockRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateBlockRequest;

        /**
         * Verifies an UpdateBlockRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateBlockRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateBlockRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateBlockRequest;

        /**
         * Creates a plain object from an UpdateBlockRequest message. Also converts values to other types if specified.
         * @param message UpdateBlockRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateBlockRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateBlockRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateBlockRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateBlockResponse. */
    interface IUpdateBlockResponse {

        /** UpdateBlockResponse block */
        block?: (petajournal.IBlock|null);
    }

    /** Represents an UpdateBlockResponse. */
    class UpdateBlockResponse implements IUpdateBlockResponse {

        /**
         * Constructs a new UpdateBlockResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateBlockResponse);

        /** UpdateBlockResponse block. */
        public block?: (petajournal.IBlock|null);

        /**
         * Creates a new UpdateBlockResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateBlockResponse instance
         */
        public static create(properties?: petajournal.IUpdateBlockResponse): petajournal.UpdateBlockResponse;

        /**
         * Encodes the specified UpdateBlockResponse message. Does not implicitly {@link petajournal.UpdateBlockResponse.verify|verify} messages.
         * @param message UpdateBlockResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateBlockResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateBlockResponse message, length delimited. Does not implicitly {@link petajournal.UpdateBlockResponse.verify|verify} messages.
         * @param message UpdateBlockResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateBlockResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateBlockResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockResponse;

        /**
         * Decodes an UpdateBlockResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateBlockResponse;

        /**
         * Verifies an UpdateBlockResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateBlockResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateBlockResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateBlockResponse;

        /**
         * Creates a plain object from an UpdateBlockResponse message. Also converts values to other types if specified.
         * @param message UpdateBlockResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateBlockResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateBlockResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateBlockResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MoveBlocksRequest. */
    interface IMoveBlocksRequest {

        /** MoveBlocksRequest blockIds */
        blockIds?: (string[]|null);

        /** MoveBlocksRequest targetPageId */
        targetPageId?: (string|null);

        /** MoveBlocksRequest afterBlockId */
        afterBlockId?: (string|null);

        /** MoveBlocksRequest levelDelta */
        levelDelta?: (number|null);
    }

    /** Represents a MoveBlocksRequest. */
    class MoveBlocksRequest implements IMoveBlocksRequest {

        /**
         * Constructs a new MoveBlocksRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IMoveBlocksRequest);

        /** MoveBlocksRequest blockIds. */
        public blockIds: string[];

        /** MoveBlocksRequest targetPageId. */
        public targetPageId: string;

        /** MoveBlocksRequest afterBlockId. */
        public afterBlockId: string;

        /** MoveBlocksRequest levelDelta. */
        public levelDelta?: (number|null);

        /**
         * Creates a new MoveBlocksRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MoveBlocksRequest instance
         */
        public static create(properties?: petajournal.IMoveBlocksRequest): petajournal.MoveBlocksRequest;

        /**
         * Encodes the specified MoveBlocksRequest message. Does not implicitly {@link petajournal.MoveBlocksRequest.verify|verify} messages.
         * @param message MoveBlocksRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IMoveBlocksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MoveBlocksRequest message, length delimited. Does not implicitly {@link petajournal.MoveBlocksRequest.verify|verify} messages.
         * @param message MoveBlocksRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IMoveBlocksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MoveBlocksRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBlocksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBlocksRequest;

        /**
         * Decodes a MoveBlocksRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MoveBlocksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.MoveBlocksRequest;

        /**
         * Verifies a MoveBlocksRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MoveBlocksRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MoveBlocksRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.MoveBlocksRequest;

        /**
         * Creates a plain object from a MoveBlocksRequest message. Also converts values to other types if specified.
         * @param message MoveBlocksRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.MoveBlocksRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MoveBlocksRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MoveBlocksRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MoveBlocksResponse. */
    interface IMoveBlocksResponse {

        /** MoveBlocksResponse blocks */
        blocks?: (petajournal.IBlock[]|null);
    }

    /** Represents a MoveBlocksResponse. */
    class MoveBlocksResponse implements IMoveBlocksResponse {

        /**
         * Constructs a new MoveBlocksResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IMoveBlocksResponse);

        /** MoveBlocksResponse blocks. */
        public blocks: petajournal.IBlock[];

        /**
         * Creates a new MoveBlocksResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MoveBlocksResponse instance
         */
        public static create(properties?: petajournal.IMoveBlocksResponse): petajournal.MoveBlocksResponse;

        /**
         * Encodes the specified MoveBlocksResponse message. Does not implicitly {@link petajournal.MoveBlocksResponse.verify|verify} messages.
         * @param message MoveBlocksResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IMoveBlocksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MoveBlocksResponse message, length delimited. Does not implicitly {@link petajournal.MoveBlocksResponse.verify|verify} messages.
         * @param message MoveBlocksResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IMoveBlocksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MoveBlocksResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBlocksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBlocksResponse;

        /**
         * Decodes a MoveBlocksResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MoveBlocksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.MoveBlocksResponse;

        /**
         * Verifies a MoveBlocksResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MoveBlocksResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MoveBlocksResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.MoveBlocksResponse;

        /**
         * Creates a plain object from a MoveBlocksResponse message. Also converts values to other types if specified.
         * @param message MoveBlocksResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.MoveBlocksResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MoveBlocksResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MoveBlocksResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeleteBlockRequest. */
    interface IDeleteBlockRequest {

        /** DeleteBlockRequest blockId */
        blockId?: (string|null);
    }

    /** Represents a DeleteBlockRequest. */
    class DeleteBlockRequest implements IDeleteBlockRequest {

        /**
         * Constructs a new DeleteBlockRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeleteBlockRequest);

        /** DeleteBlockRequest blockId. */
        public blockId: string;

        /**
         * Creates a new DeleteBlockRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeleteBlockRequest instance
         */
        public static create(properties?: petajournal.IDeleteBlockRequest): petajournal.DeleteBlockRequest;

        /**
         * Encodes the specified DeleteBlockRequest message. Does not implicitly {@link petajournal.DeleteBlockRequest.verify|verify} messages.
         * @param message DeleteBlockRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeleteBlockRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeleteBlockRequest message, length delimited. Does not implicitly {@link petajournal.DeleteBlockRequest.verify|verify} messages.
         * @param message DeleteBlockRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeleteBlockRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeleteBlockRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBlockRequest;

        /**
         * Decodes a DeleteBlockRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeleteBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeleteBlockRequest;

        /**
         * Verifies a DeleteBlockRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeleteBlockRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeleteBlockRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeleteBlockRequest;

        /**
         * Creates a plain object from a DeleteBlockRequest message. Also converts values to other types if specified.
         * @param message DeleteBlockRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeleteBlockRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeleteBlockRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeleteBlockRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeleteBlockResponse. */
    interface IDeleteBlockResponse {
    }

    /** Represents a DeleteBlockResponse. */
    class DeleteBlockResponse implements IDeleteBlockResponse {

        /**
         * Constructs a new DeleteBlockResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeleteBlockResponse);

        /**
         * Creates a new DeleteBlockResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeleteBlockResponse instance
         */
        public static create(properties?: petajournal.IDeleteBlockResponse): petajournal.DeleteBlockResponse;

        /**
         * Encodes the specified DeleteBlockResponse message. Does not implicitly {@link petajournal.DeleteBlockResponse.verify|verify} messages.
         * @param message DeleteBlockResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeleteBlockResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeleteBlockResponse message, length delimited. Does not implicitly {@link petajournal.DeleteBlockResponse.verify|verify} messages.
         * @param message DeleteBlockResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeleteBlockResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeleteBlockResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBlockResponse;

        /**
         * Decodes a DeleteBlockResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeleteBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeleteBlockResponse;

        /**
         * Verifies a DeleteBlockResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeleteBlockResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeleteBlockResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeleteBlockResponse;

        /**
         * Creates a plain object from a DeleteBlockResponse message. Also converts values to other types if specified.
         * @param message DeleteBlockResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeleteBlockResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeleteBlockResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeleteBlockResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateBlockStatusRequest. */
    interface IUpdateBlockStatusRequest {

        /** UpdateBlockStatusRequest blockId */
        blockId?: (string|null);

        /** UpdateBlockStatusRequest status */
        status?: (string|null);
    }

    /** Represents an UpdateBlockStatusRequest. */
    class UpdateBlockStatusRequest implements IUpdateBlockStatusRequest {

        /**
         * Constructs a new UpdateBlockStatusRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateBlockStatusRequest);

        /** UpdateBlockStatusRequest blockId. */
        public blockId: string;

        /** UpdateBlockStatusRequest status. */
        public status: string;

        /**
         * Creates a new UpdateBlockStatusRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateBlockStatusRequest instance
         */
        public static create(properties?: petajournal.IUpdateBlockStatusRequest): petajournal.UpdateBlockStatusRequest;

        /**
         * Encodes the specified UpdateBlockStatusRequest message. Does not implicitly {@link petajournal.UpdateBlockStatusRequest.verify|verify} messages.
         * @param message UpdateBlockStatusRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateBlockStatusRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateBlockStatusRequest message, length delimited. Does not implicitly {@link petajournal.UpdateBlockStatusRequest.verify|verify} messages.
         * @param message UpdateBlockStatusRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateBlockStatusRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateBlockStatusRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockStatusRequest;

        /**
         * Decodes an UpdateBlockStatusRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateBlockStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateBlockStatusRequest;

        /**
         * Verifies an UpdateBlockStatusRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateBlockStatusRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateBlockStatusRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateBlockStatusRequest;

        /**
         * Creates a plain object from an UpdateBlockStatusRequest message. Also converts values to other types if specified.
         * @param message UpdateBlockStatusRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateBlockStatusRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateBlockStatusRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateBlockStatusRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateBlockStatusResponse. */
    interface IUpdateBlockStatusResponse {

        /** UpdateBlockStatusResponse block */
        block?: (petajournal.IBlock|null);
    }

    /** Represents an UpdateBlockStatusResponse. */
    class UpdateBlockStatusResponse implements IUpdateBlockStatusResponse {

        /**
         * Constructs a new UpdateBlockStatusResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateBlockStatusResponse);

        /** UpdateBlockStatusResponse block. */
        public block?: (petajournal.IBlock|null);

        /**
         * Creates a new UpdateBlockStatusResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateBlockStatusResponse instance
         */
        public static create(properties?: petajournal.IUpdateBlockStatusResponse): petajournal.UpdateBlockStatusResponse;

        /**
         * Encodes the specified UpdateBlockStatusResponse message. Does not implicitly {@link petajournal.UpdateBlockStatusResponse.verify|verify} messages.
         * @param message UpdateBlockStatusResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateBlockStatusResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateBlockStatusResponse message, length delimited. Does not implicitly {@link petajournal.UpdateBlockStatusResponse.verify|verify} messages.
         * @param message UpdateBlockStatusResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateBlockStatusResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateBlockStatusResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockStatusResponse;

        /**
         * Decodes an UpdateBlockStatusResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateBlockStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateBlockStatusResponse;

        /**
         * Verifies an UpdateBlockStatusResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateBlockStatusResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateBlockStatusResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateBlockStatusResponse;

        /**
         * Creates a plain object from an UpdateBlockStatusResponse message. Also converts values to other types if specified.
         * @param message UpdateBlockStatusResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateBlockStatusResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateBlockStatusResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateBlockStatusResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchPageNamesRequest. */
    interface ISearchPageNamesRequest {

        /** SearchPageNamesRequest query */
        query?: (string|null);

        /** SearchPageNamesRequest maxResults */
        maxResults?: (number|null);
    }

    /** Represents a SearchPageNamesRequest. */
    class SearchPageNamesRequest implements ISearchPageNamesRequest {

        /**
         * Constructs a new SearchPageNamesRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchPageNamesRequest);

        /** SearchPageNamesRequest query. */
        public query: string;

        /** SearchPageNamesRequest maxResults. */
        public maxResults: number;

        /**
         * Creates a new SearchPageNamesRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchPageNamesRequest instance
         */
        public static create(properties?: petajournal.ISearchPageNamesRequest): petajournal.SearchPageNamesRequest;

        /**
         * Encodes the specified SearchPageNamesRequest message. Does not implicitly {@link petajournal.SearchPageNamesRequest.verify|verify} messages.
         * @param message SearchPageNamesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchPageNamesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchPageNamesRequest message, length delimited. Does not implicitly {@link petajournal.SearchPageNamesRequest.verify|verify} messages.
         * @param message SearchPageNamesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchPageNamesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchPageNamesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchPageNamesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchPageNamesRequest;

        /**
         * Decodes a SearchPageNamesRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchPageNamesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchPageNamesRequest;

        /**
         * Verifies a SearchPageNamesRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchPageNamesRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchPageNamesRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchPageNamesRequest;

        /**
         * Creates a plain object from a SearchPageNamesRequest message. Also converts values to other types if specified.
         * @param message SearchPageNamesRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchPageNamesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchPageNamesRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchPageNamesRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchPageNamesResponse. */
    interface ISearchPageNamesResponse {

        /** SearchPageNamesResponse results */
        results?: (petajournal.ISearchResult[]|null);
    }

    /** Represents a SearchPageNamesResponse. */
    class SearchPageNamesResponse implements ISearchPageNamesResponse {

        /**
         * Constructs a new SearchPageNamesResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchPageNamesResponse);

        /** SearchPageNamesResponse results. */
        public results: petajournal.ISearchResult[];

        /**
         * Creates a new SearchPageNamesResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchPageNamesResponse instance
         */
        public static create(properties?: petajournal.ISearchPageNamesResponse): petajournal.SearchPageNamesResponse;

        /**
         * Encodes the specified SearchPageNamesResponse message. Does not implicitly {@link petajournal.SearchPageNamesResponse.verify|verify} messages.
         * @param message SearchPageNamesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchPageNamesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchPageNamesResponse message, length delimited. Does not implicitly {@link petajournal.SearchPageNamesResponse.verify|verify} messages.
         * @param message SearchPageNamesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchPageNamesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchPageNamesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchPageNamesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchPageNamesResponse;

        /**
         * Decodes a SearchPageNamesResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchPageNamesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchPageNamesResponse;

        /**
         * Verifies a SearchPageNamesResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchPageNamesResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchPageNamesResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchPageNamesResponse;

        /**
         * Creates a plain object from a SearchPageNamesResponse message. Also converts values to other types if specified.
         * @param message SearchPageNamesResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchPageNamesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchPageNamesResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchPageNamesResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchFullTextRequest. */
    interface ISearchFullTextRequest {

        /** SearchFullTextRequest query */
        query?: (string|null);

        /** SearchFullTextRequest maxResults */
        maxResults?: (number|null);
    }

    /** Represents a SearchFullTextRequest. */
    class SearchFullTextRequest implements ISearchFullTextRequest {

        /**
         * Constructs a new SearchFullTextRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchFullTextRequest);

        /** SearchFullTextRequest query. */
        public query: string;

        /** SearchFullTextRequest maxResults. */
        public maxResults: number;

        /**
         * Creates a new SearchFullTextRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchFullTextRequest instance
         */
        public static create(properties?: petajournal.ISearchFullTextRequest): petajournal.SearchFullTextRequest;

        /**
         * Encodes the specified SearchFullTextRequest message. Does not implicitly {@link petajournal.SearchFullTextRequest.verify|verify} messages.
         * @param message SearchFullTextRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchFullTextRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchFullTextRequest message, length delimited. Does not implicitly {@link petajournal.SearchFullTextRequest.verify|verify} messages.
         * @param message SearchFullTextRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchFullTextRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchFullTextRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchFullTextRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchFullTextRequest;

        /**
         * Decodes a SearchFullTextRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchFullTextRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchFullTextRequest;

        /**
         * Verifies a SearchFullTextRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchFullTextRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchFullTextRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchFullTextRequest;

        /**
         * Creates a plain object from a SearchFullTextRequest message. Also converts values to other types if specified.
         * @param message SearchFullTextRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchFullTextRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchFullTextRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchFullTextRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SearchFullTextResponse. */
    interface ISearchFullTextResponse {

        /** SearchFullTextResponse results */
        results?: (petajournal.ISearchResult[]|null);

        /** SearchFullTextResponse totalHits */
        totalHits?: (number|null);
    }

    /** Represents a SearchFullTextResponse. */
    class SearchFullTextResponse implements ISearchFullTextResponse {

        /**
         * Constructs a new SearchFullTextResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ISearchFullTextResponse);

        /** SearchFullTextResponse results. */
        public results: petajournal.ISearchResult[];

        /** SearchFullTextResponse totalHits. */
        public totalHits: number;

        /**
         * Creates a new SearchFullTextResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchFullTextResponse instance
         */
        public static create(properties?: petajournal.ISearchFullTextResponse): petajournal.SearchFullTextResponse;

        /**
         * Encodes the specified SearchFullTextResponse message. Does not implicitly {@link petajournal.SearchFullTextResponse.verify|verify} messages.
         * @param message SearchFullTextResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ISearchFullTextResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchFullTextResponse message, length delimited. Does not implicitly {@link petajournal.SearchFullTextResponse.verify|verify} messages.
         * @param message SearchFullTextResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ISearchFullTextResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchFullTextResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchFullTextResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchFullTextResponse;

        /**
         * Decodes a SearchFullTextResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchFullTextResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.SearchFullTextResponse;

        /**
         * Verifies a SearchFullTextResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchFullTextResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchFullTextResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.SearchFullTextResponse;

        /**
         * Creates a plain object from a SearchFullTextResponse message. Also converts values to other types if specified.
         * @param message SearchFullTextResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.SearchFullTextResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchFullTextResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SearchFullTextResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetRecentPagesRequest. */
    interface IGetRecentPagesRequest {

        /** GetRecentPagesRequest maxResults */
        maxResults?: (number|null);

        /** GetRecentPagesRequest type */
        type?: (string|null);
    }

    /** Represents a GetRecentPagesRequest. */
    class GetRecentPagesRequest implements IGetRecentPagesRequest {

        /**
         * Constructs a new GetRecentPagesRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetRecentPagesRequest);

        /** GetRecentPagesRequest maxResults. */
        public maxResults: number;

        /** GetRecentPagesRequest type. */
        public type: string;

        /**
         * Creates a new GetRecentPagesRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetRecentPagesRequest instance
         */
        public static create(properties?: petajournal.IGetRecentPagesRequest): petajournal.GetRecentPagesRequest;

        /**
         * Encodes the specified GetRecentPagesRequest message. Does not implicitly {@link petajournal.GetRecentPagesRequest.verify|verify} messages.
         * @param message GetRecentPagesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetRecentPagesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetRecentPagesRequest message, length delimited. Does not implicitly {@link petajournal.GetRecentPagesRequest.verify|verify} messages.
         * @param message GetRecentPagesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetRecentPagesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetRecentPagesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetRecentPagesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetRecentPagesRequest;

        /**
         * Decodes a GetRecentPagesRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetRecentPagesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetRecentPagesRequest;

        /**
         * Verifies a GetRecentPagesRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetRecentPagesRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetRecentPagesRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetRecentPagesRequest;

        /**
         * Creates a plain object from a GetRecentPagesRequest message. Also converts values to other types if specified.
         * @param message GetRecentPagesRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetRecentPagesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetRecentPagesRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetRecentPagesRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetRecentPagesResponse. */
    interface IGetRecentPagesResponse {

        /** GetRecentPagesResponse pages */
        pages?: (petajournal.IPage[]|null);
    }

    /** Represents a GetRecentPagesResponse. */
    class GetRecentPagesResponse implements IGetRecentPagesResponse {

        /**
         * Constructs a new GetRecentPagesResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetRecentPagesResponse);

        /** GetRecentPagesResponse pages. */
        public pages: petajournal.IPage[];

        /**
         * Creates a new GetRecentPagesResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetRecentPagesResponse instance
         */
        public static create(properties?: petajournal.IGetRecentPagesResponse): petajournal.GetRecentPagesResponse;

        /**
         * Encodes the specified GetRecentPagesResponse message. Does not implicitly {@link petajournal.GetRecentPagesResponse.verify|verify} messages.
         * @param message GetRecentPagesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetRecentPagesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetRecentPagesResponse message, length delimited. Does not implicitly {@link petajournal.GetRecentPagesResponse.verify|verify} messages.
         * @param message GetRecentPagesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetRecentPagesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetRecentPagesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetRecentPagesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetRecentPagesResponse;

        /**
         * Decodes a GetRecentPagesResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetRecentPagesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetRecentPagesResponse;

        /**
         * Verifies a GetRecentPagesResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetRecentPagesResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetRecentPagesResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetRecentPagesResponse;

        /**
         * Creates a plain object from a GetRecentPagesResponse message. Also converts values to other types if specified.
         * @param message GetRecentPagesResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetRecentPagesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetRecentPagesResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetRecentPagesResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetBacklinksRequest. */
    interface IGetBacklinksRequest {

        /** GetBacklinksRequest pageId */
        pageId?: (string|null);
    }

    /** Represents a GetBacklinksRequest. */
    class GetBacklinksRequest implements IGetBacklinksRequest {

        /**
         * Constructs a new GetBacklinksRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetBacklinksRequest);

        /** GetBacklinksRequest pageId. */
        public pageId: string;

        /**
         * Creates a new GetBacklinksRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetBacklinksRequest instance
         */
        public static create(properties?: petajournal.IGetBacklinksRequest): petajournal.GetBacklinksRequest;

        /**
         * Encodes the specified GetBacklinksRequest message. Does not implicitly {@link petajournal.GetBacklinksRequest.verify|verify} messages.
         * @param message GetBacklinksRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetBacklinksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetBacklinksRequest message, length delimited. Does not implicitly {@link petajournal.GetBacklinksRequest.verify|verify} messages.
         * @param message GetBacklinksRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetBacklinksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetBacklinksRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBacklinksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBacklinksRequest;

        /**
         * Decodes a GetBacklinksRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetBacklinksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetBacklinksRequest;

        /**
         * Verifies a GetBacklinksRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetBacklinksRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetBacklinksRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetBacklinksRequest;

        /**
         * Creates a plain object from a GetBacklinksRequest message. Also converts values to other types if specified.
         * @param message GetBacklinksRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetBacklinksRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetBacklinksRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetBacklinksRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetBacklinksResponse. */
    interface IGetBacklinksResponse {

        /** GetBacklinksResponse entries */
        entries?: (petajournal.IBacklinkEntry[]|null);
    }

    /** Represents a GetBacklinksResponse. */
    class GetBacklinksResponse implements IGetBacklinksResponse {

        /**
         * Constructs a new GetBacklinksResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetBacklinksResponse);

        /** GetBacklinksResponse entries. */
        public entries: petajournal.IBacklinkEntry[];

        /**
         * Creates a new GetBacklinksResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetBacklinksResponse instance
         */
        public static create(properties?: petajournal.IGetBacklinksResponse): petajournal.GetBacklinksResponse;

        /**
         * Encodes the specified GetBacklinksResponse message. Does not implicitly {@link petajournal.GetBacklinksResponse.verify|verify} messages.
         * @param message GetBacklinksResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetBacklinksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetBacklinksResponse message, length delimited. Does not implicitly {@link petajournal.GetBacklinksResponse.verify|verify} messages.
         * @param message GetBacklinksResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetBacklinksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetBacklinksResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBacklinksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBacklinksResponse;

        /**
         * Decodes a GetBacklinksResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetBacklinksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetBacklinksResponse;

        /**
         * Verifies a GetBacklinksResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetBacklinksResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetBacklinksResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetBacklinksResponse;

        /**
         * Creates a plain object from a GetBacklinksResponse message. Also converts values to other types if specified.
         * @param message GetBacklinksResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetBacklinksResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetBacklinksResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetBacklinksResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a BacklinkEntry. */
    interface IBacklinkEntry {

        /** BacklinkEntry sourcePage */
        sourcePage?: (petajournal.IPage|null);

        /** BacklinkEntry sourceBlock */
        sourceBlock?: (petajournal.IBlock|null);
    }

    /** Represents a BacklinkEntry. */
    class BacklinkEntry implements IBacklinkEntry {

        /**
         * Constructs a new BacklinkEntry.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IBacklinkEntry);

        /** BacklinkEntry sourcePage. */
        public sourcePage?: (petajournal.IPage|null);

        /** BacklinkEntry sourceBlock. */
        public sourceBlock?: (petajournal.IBlock|null);

        /**
         * Creates a new BacklinkEntry instance using the specified properties.
         * @param [properties] Properties to set
         * @returns BacklinkEntry instance
         */
        public static create(properties?: petajournal.IBacklinkEntry): petajournal.BacklinkEntry;

        /**
         * Encodes the specified BacklinkEntry message. Does not implicitly {@link petajournal.BacklinkEntry.verify|verify} messages.
         * @param message BacklinkEntry message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IBacklinkEntry, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified BacklinkEntry message, length delimited. Does not implicitly {@link petajournal.BacklinkEntry.verify|verify} messages.
         * @param message BacklinkEntry message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IBacklinkEntry, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a BacklinkEntry message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns BacklinkEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.BacklinkEntry;

        /**
         * Decodes a BacklinkEntry message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns BacklinkEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.BacklinkEntry;

        /**
         * Verifies a BacklinkEntry message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a BacklinkEntry message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns BacklinkEntry
         */
        public static fromObject(object: { [k: string]: any }): petajournal.BacklinkEntry;

        /**
         * Creates a plain object from a BacklinkEntry message. Also converts values to other types if specified.
         * @param message BacklinkEntry
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.BacklinkEntry, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this BacklinkEntry to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for BacklinkEntry
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetMostRecentJournalBeforeRequest. */
    interface IGetMostRecentJournalBeforeRequest {

        /** GetMostRecentJournalBeforeRequest beforeDate */
        beforeDate?: (string|null);
    }

    /** Represents a GetMostRecentJournalBeforeRequest. */
    class GetMostRecentJournalBeforeRequest implements IGetMostRecentJournalBeforeRequest {

        /**
         * Constructs a new GetMostRecentJournalBeforeRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetMostRecentJournalBeforeRequest);

        /** GetMostRecentJournalBeforeRequest beforeDate. */
        public beforeDate: string;

        /**
         * Creates a new GetMostRecentJournalBeforeRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetMostRecentJournalBeforeRequest instance
         */
        public static create(properties?: petajournal.IGetMostRecentJournalBeforeRequest): petajournal.GetMostRecentJournalBeforeRequest;

        /**
         * Encodes the specified GetMostRecentJournalBeforeRequest message. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeRequest.verify|verify} messages.
         * @param message GetMostRecentJournalBeforeRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetMostRecentJournalBeforeRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetMostRecentJournalBeforeRequest message, length delimited. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeRequest.verify|verify} messages.
         * @param message GetMostRecentJournalBeforeRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetMostRecentJournalBeforeRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetMostRecentJournalBeforeRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetMostRecentJournalBeforeRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetMostRecentJournalBeforeRequest;

        /**
         * Decodes a GetMostRecentJournalBeforeRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetMostRecentJournalBeforeRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetMostRecentJournalBeforeRequest;

        /**
         * Verifies a GetMostRecentJournalBeforeRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetMostRecentJournalBeforeRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetMostRecentJournalBeforeRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetMostRecentJournalBeforeRequest;

        /**
         * Creates a plain object from a GetMostRecentJournalBeforeRequest message. Also converts values to other types if specified.
         * @param message GetMostRecentJournalBeforeRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetMostRecentJournalBeforeRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetMostRecentJournalBeforeRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetMostRecentJournalBeforeRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetMostRecentJournalBeforeResponse. */
    interface IGetMostRecentJournalBeforeResponse {

        /** GetMostRecentJournalBeforeResponse found */
        found?: (boolean|null);

        /** GetMostRecentJournalBeforeResponse page */
        page?: (petajournal.IPage|null);

        /** GetMostRecentJournalBeforeResponse blocks */
        blocks?: (petajournal.IBlock[]|null);
    }

    /** Represents a GetMostRecentJournalBeforeResponse. */
    class GetMostRecentJournalBeforeResponse implements IGetMostRecentJournalBeforeResponse {

        /**
         * Constructs a new GetMostRecentJournalBeforeResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetMostRecentJournalBeforeResponse);

        /** GetMostRecentJournalBeforeResponse found. */
        public found: boolean;

        /** GetMostRecentJournalBeforeResponse page. */
        public page?: (petajournal.IPage|null);

        /** GetMostRecentJournalBeforeResponse blocks. */
        public blocks: petajournal.IBlock[];

        /**
         * Creates a new GetMostRecentJournalBeforeResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetMostRecentJournalBeforeResponse instance
         */
        public static create(properties?: petajournal.IGetMostRecentJournalBeforeResponse): petajournal.GetMostRecentJournalBeforeResponse;

        /**
         * Encodes the specified GetMostRecentJournalBeforeResponse message. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeResponse.verify|verify} messages.
         * @param message GetMostRecentJournalBeforeResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetMostRecentJournalBeforeResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetMostRecentJournalBeforeResponse message, length delimited. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeResponse.verify|verify} messages.
         * @param message GetMostRecentJournalBeforeResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetMostRecentJournalBeforeResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetMostRecentJournalBeforeResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetMostRecentJournalBeforeResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetMostRecentJournalBeforeResponse;

        /**
         * Decodes a GetMostRecentJournalBeforeResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetMostRecentJournalBeforeResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetMostRecentJournalBeforeResponse;

        /**
         * Verifies a GetMostRecentJournalBeforeResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetMostRecentJournalBeforeResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetMostRecentJournalBeforeResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetMostRecentJournalBeforeResponse;

        /**
         * Creates a plain object from a GetMostRecentJournalBeforeResponse message. Also converts values to other types if specified.
         * @param message GetMostRecentJournalBeforeResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetMostRecentJournalBeforeResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetMostRecentJournalBeforeResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetMostRecentJournalBeforeResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetBlocksByStatusRequest. */
    interface IGetBlocksByStatusRequest {

        /** GetBlocksByStatusRequest statuses */
        statuses?: (string[]|null);
    }

    /** Represents a GetBlocksByStatusRequest. */
    class GetBlocksByStatusRequest implements IGetBlocksByStatusRequest {

        /**
         * Constructs a new GetBlocksByStatusRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetBlocksByStatusRequest);

        /** GetBlocksByStatusRequest statuses. */
        public statuses: string[];

        /**
         * Creates a new GetBlocksByStatusRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetBlocksByStatusRequest instance
         */
        public static create(properties?: petajournal.IGetBlocksByStatusRequest): petajournal.GetBlocksByStatusRequest;

        /**
         * Encodes the specified GetBlocksByStatusRequest message. Does not implicitly {@link petajournal.GetBlocksByStatusRequest.verify|verify} messages.
         * @param message GetBlocksByStatusRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetBlocksByStatusRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetBlocksByStatusRequest message, length delimited. Does not implicitly {@link petajournal.GetBlocksByStatusRequest.verify|verify} messages.
         * @param message GetBlocksByStatusRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetBlocksByStatusRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetBlocksByStatusRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBlocksByStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBlocksByStatusRequest;

        /**
         * Decodes a GetBlocksByStatusRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetBlocksByStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetBlocksByStatusRequest;

        /**
         * Verifies a GetBlocksByStatusRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetBlocksByStatusRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetBlocksByStatusRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetBlocksByStatusRequest;

        /**
         * Creates a plain object from a GetBlocksByStatusRequest message. Also converts values to other types if specified.
         * @param message GetBlocksByStatusRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetBlocksByStatusRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetBlocksByStatusRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetBlocksByStatusRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetBlocksByStatusResponse. */
    interface IGetBlocksByStatusResponse {

        /** GetBlocksByStatusResponse blocks */
        blocks?: (petajournal.IBlock[]|null);
    }

    /** Represents a GetBlocksByStatusResponse. */
    class GetBlocksByStatusResponse implements IGetBlocksByStatusResponse {

        /**
         * Constructs a new GetBlocksByStatusResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetBlocksByStatusResponse);

        /** GetBlocksByStatusResponse blocks. */
        public blocks: petajournal.IBlock[];

        /**
         * Creates a new GetBlocksByStatusResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetBlocksByStatusResponse instance
         */
        public static create(properties?: petajournal.IGetBlocksByStatusResponse): petajournal.GetBlocksByStatusResponse;

        /**
         * Encodes the specified GetBlocksByStatusResponse message. Does not implicitly {@link petajournal.GetBlocksByStatusResponse.verify|verify} messages.
         * @param message GetBlocksByStatusResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetBlocksByStatusResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetBlocksByStatusResponse message, length delimited. Does not implicitly {@link petajournal.GetBlocksByStatusResponse.verify|verify} messages.
         * @param message GetBlocksByStatusResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetBlocksByStatusResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetBlocksByStatusResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBlocksByStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBlocksByStatusResponse;

        /**
         * Decodes a GetBlocksByStatusResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetBlocksByStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetBlocksByStatusResponse;

        /**
         * Verifies a GetBlocksByStatusResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetBlocksByStatusResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetBlocksByStatusResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetBlocksByStatusResponse;

        /**
         * Creates a plain object from a GetBlocksByStatusResponse message. Also converts values to other types if specified.
         * @param message GetBlocksByStatusResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetBlocksByStatusResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetBlocksByStatusResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetBlocksByStatusResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetOrCreateJournalRequest. */
    interface IGetOrCreateJournalRequest {

        /** GetOrCreateJournalRequest name */
        name?: (string|null);
    }

    /** Represents a GetOrCreateJournalRequest. */
    class GetOrCreateJournalRequest implements IGetOrCreateJournalRequest {

        /**
         * Constructs a new GetOrCreateJournalRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetOrCreateJournalRequest);

        /** GetOrCreateJournalRequest name. */
        public name: string;

        /**
         * Creates a new GetOrCreateJournalRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetOrCreateJournalRequest instance
         */
        public static create(properties?: petajournal.IGetOrCreateJournalRequest): petajournal.GetOrCreateJournalRequest;

        /**
         * Encodes the specified GetOrCreateJournalRequest message. Does not implicitly {@link petajournal.GetOrCreateJournalRequest.verify|verify} messages.
         * @param message GetOrCreateJournalRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetOrCreateJournalRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetOrCreateJournalRequest message, length delimited. Does not implicitly {@link petajournal.GetOrCreateJournalRequest.verify|verify} messages.
         * @param message GetOrCreateJournalRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetOrCreateJournalRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetOrCreateJournalRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetOrCreateJournalRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetOrCreateJournalRequest;

        /**
         * Decodes a GetOrCreateJournalRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetOrCreateJournalRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetOrCreateJournalRequest;

        /**
         * Verifies a GetOrCreateJournalRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetOrCreateJournalRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetOrCreateJournalRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetOrCreateJournalRequest;

        /**
         * Creates a plain object from a GetOrCreateJournalRequest message. Also converts values to other types if specified.
         * @param message GetOrCreateJournalRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetOrCreateJournalRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetOrCreateJournalRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetOrCreateJournalRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetOrCreateJournalResponse. */
    interface IGetOrCreateJournalResponse {

        /** GetOrCreateJournalResponse page */
        page?: (petajournal.IPage|null);

        /** GetOrCreateJournalResponse blocks */
        blocks?: (petajournal.IBlock[]|null);

        /** GetOrCreateJournalResponse created */
        created?: (boolean|null);
    }

    /** Represents a GetOrCreateJournalResponse. */
    class GetOrCreateJournalResponse implements IGetOrCreateJournalResponse {

        /**
         * Constructs a new GetOrCreateJournalResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetOrCreateJournalResponse);

        /** GetOrCreateJournalResponse page. */
        public page?: (petajournal.IPage|null);

        /** GetOrCreateJournalResponse blocks. */
        public blocks: petajournal.IBlock[];

        /** GetOrCreateJournalResponse created. */
        public created: boolean;

        /**
         * Creates a new GetOrCreateJournalResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetOrCreateJournalResponse instance
         */
        public static create(properties?: petajournal.IGetOrCreateJournalResponse): petajournal.GetOrCreateJournalResponse;

        /**
         * Encodes the specified GetOrCreateJournalResponse message. Does not implicitly {@link petajournal.GetOrCreateJournalResponse.verify|verify} messages.
         * @param message GetOrCreateJournalResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetOrCreateJournalResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetOrCreateJournalResponse message, length delimited. Does not implicitly {@link petajournal.GetOrCreateJournalResponse.verify|verify} messages.
         * @param message GetOrCreateJournalResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetOrCreateJournalResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetOrCreateJournalResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetOrCreateJournalResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetOrCreateJournalResponse;

        /**
         * Decodes a GetOrCreateJournalResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetOrCreateJournalResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetOrCreateJournalResponse;

        /**
         * Verifies a GetOrCreateJournalResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetOrCreateJournalResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetOrCreateJournalResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetOrCreateJournalResponse;

        /**
         * Creates a plain object from a GetOrCreateJournalResponse message. Also converts values to other types if specified.
         * @param message GetOrCreateJournalResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetOrCreateJournalResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetOrCreateJournalResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetOrCreateJournalResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetConfigRequest. */
    interface IGetConfigRequest {
    }

    /** Represents a GetConfigRequest. */
    class GetConfigRequest implements IGetConfigRequest {

        /**
         * Constructs a new GetConfigRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetConfigRequest);

        /**
         * Creates a new GetConfigRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetConfigRequest instance
         */
        public static create(properties?: petajournal.IGetConfigRequest): petajournal.GetConfigRequest;

        /**
         * Encodes the specified GetConfigRequest message. Does not implicitly {@link petajournal.GetConfigRequest.verify|verify} messages.
         * @param message GetConfigRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetConfigRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetConfigRequest message, length delimited. Does not implicitly {@link petajournal.GetConfigRequest.verify|verify} messages.
         * @param message GetConfigRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetConfigRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetConfigRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetConfigRequest;

        /**
         * Decodes a GetConfigRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetConfigRequest;

        /**
         * Verifies a GetConfigRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetConfigRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetConfigRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetConfigRequest;

        /**
         * Creates a plain object from a GetConfigRequest message. Also converts values to other types if specified.
         * @param message GetConfigRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetConfigRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetConfigRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetConfigRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetConfigResponse. */
    interface IGetConfigResponse {

        /** GetConfigResponse weekNumbering */
        weekNumbering?: (string|null);
    }

    /** Represents a GetConfigResponse. */
    class GetConfigResponse implements IGetConfigResponse {

        /**
         * Constructs a new GetConfigResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetConfigResponse);

        /** GetConfigResponse weekNumbering. */
        public weekNumbering: string;

        /**
         * Creates a new GetConfigResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetConfigResponse instance
         */
        public static create(properties?: petajournal.IGetConfigResponse): petajournal.GetConfigResponse;

        /**
         * Encodes the specified GetConfigResponse message. Does not implicitly {@link petajournal.GetConfigResponse.verify|verify} messages.
         * @param message GetConfigResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetConfigResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetConfigResponse message, length delimited. Does not implicitly {@link petajournal.GetConfigResponse.verify|verify} messages.
         * @param message GetConfigResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetConfigResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetConfigResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetConfigResponse;

        /**
         * Decodes a GetConfigResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetConfigResponse;

        /**
         * Verifies a GetConfigResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetConfigResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetConfigResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetConfigResponse;

        /**
         * Creates a plain object from a GetConfigResponse message. Also converts values to other types if specified.
         * @param message GetConfigResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetConfigResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetConfigResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetConfigResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateConfigRequest. */
    interface IUpdateConfigRequest {

        /** UpdateConfigRequest weekNumbering */
        weekNumbering?: (string|null);
    }

    /** Represents an UpdateConfigRequest. */
    class UpdateConfigRequest implements IUpdateConfigRequest {

        /**
         * Constructs a new UpdateConfigRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateConfigRequest);

        /** UpdateConfigRequest weekNumbering. */
        public weekNumbering: string;

        /**
         * Creates a new UpdateConfigRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateConfigRequest instance
         */
        public static create(properties?: petajournal.IUpdateConfigRequest): petajournal.UpdateConfigRequest;

        /**
         * Encodes the specified UpdateConfigRequest message. Does not implicitly {@link petajournal.UpdateConfigRequest.verify|verify} messages.
         * @param message UpdateConfigRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateConfigRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateConfigRequest message, length delimited. Does not implicitly {@link petajournal.UpdateConfigRequest.verify|verify} messages.
         * @param message UpdateConfigRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateConfigRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateConfigRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateConfigRequest;

        /**
         * Decodes an UpdateConfigRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateConfigRequest;

        /**
         * Verifies an UpdateConfigRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateConfigRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateConfigRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateConfigRequest;

        /**
         * Creates a plain object from an UpdateConfigRequest message. Also converts values to other types if specified.
         * @param message UpdateConfigRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateConfigRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateConfigRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateConfigRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateConfigResponse. */
    interface IUpdateConfigResponse {

        /** UpdateConfigResponse weekNumbering */
        weekNumbering?: (string|null);
    }

    /** Represents an UpdateConfigResponse. */
    class UpdateConfigResponse implements IUpdateConfigResponse {

        /**
         * Constructs a new UpdateConfigResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateConfigResponse);

        /** UpdateConfigResponse weekNumbering. */
        public weekNumbering: string;

        /**
         * Creates a new UpdateConfigResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateConfigResponse instance
         */
        public static create(properties?: petajournal.IUpdateConfigResponse): petajournal.UpdateConfigResponse;

        /**
         * Encodes the specified UpdateConfigResponse message. Does not implicitly {@link petajournal.UpdateConfigResponse.verify|verify} messages.
         * @param message UpdateConfigResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateConfigResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateConfigResponse message, length delimited. Does not implicitly {@link petajournal.UpdateConfigResponse.verify|verify} messages.
         * @param message UpdateConfigResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateConfigResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateConfigResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateConfigResponse;

        /**
         * Decodes an UpdateConfigResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateConfigResponse;

        /**
         * Verifies an UpdateConfigResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateConfigResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateConfigResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateConfigResponse;

        /**
         * Creates a plain object from an UpdateConfigResponse message. Also converts values to other types if specified.
         * @param message UpdateConfigResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateConfigResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateConfigResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateConfigResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ExportTreeMarkdownRequest. */
    interface IExportTreeMarkdownRequest {

        /** ExportTreeMarkdownRequest blockIds */
        blockIds?: (string[]|null);
    }

    /** Represents an ExportTreeMarkdownRequest. */
    class ExportTreeMarkdownRequest implements IExportTreeMarkdownRequest {

        /**
         * Constructs a new ExportTreeMarkdownRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IExportTreeMarkdownRequest);

        /** ExportTreeMarkdownRequest blockIds. */
        public blockIds: string[];

        /**
         * Creates a new ExportTreeMarkdownRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ExportTreeMarkdownRequest instance
         */
        public static create(properties?: petajournal.IExportTreeMarkdownRequest): petajournal.ExportTreeMarkdownRequest;

        /**
         * Encodes the specified ExportTreeMarkdownRequest message. Does not implicitly {@link petajournal.ExportTreeMarkdownRequest.verify|verify} messages.
         * @param message ExportTreeMarkdownRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IExportTreeMarkdownRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ExportTreeMarkdownRequest message, length delimited. Does not implicitly {@link petajournal.ExportTreeMarkdownRequest.verify|verify} messages.
         * @param message ExportTreeMarkdownRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IExportTreeMarkdownRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ExportTreeMarkdownRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ExportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ExportTreeMarkdownRequest;

        /**
         * Decodes an ExportTreeMarkdownRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ExportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ExportTreeMarkdownRequest;

        /**
         * Verifies an ExportTreeMarkdownRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ExportTreeMarkdownRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ExportTreeMarkdownRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ExportTreeMarkdownRequest;

        /**
         * Creates a plain object from an ExportTreeMarkdownRequest message. Also converts values to other types if specified.
         * @param message ExportTreeMarkdownRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ExportTreeMarkdownRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ExportTreeMarkdownRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ExportTreeMarkdownRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ExportTreeMarkdownResponse. */
    interface IExportTreeMarkdownResponse {

        /** ExportTreeMarkdownResponse treeMarkdown */
        treeMarkdown?: (string|null);

        /** ExportTreeMarkdownResponse html */
        html?: (string|null);

        /** ExportTreeMarkdownResponse flatMarkdown */
        flatMarkdown?: (string|null);
    }

    /** Represents an ExportTreeMarkdownResponse. */
    class ExportTreeMarkdownResponse implements IExportTreeMarkdownResponse {

        /**
         * Constructs a new ExportTreeMarkdownResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IExportTreeMarkdownResponse);

        /** ExportTreeMarkdownResponse treeMarkdown. */
        public treeMarkdown: string;

        /** ExportTreeMarkdownResponse html. */
        public html: string;

        /** ExportTreeMarkdownResponse flatMarkdown. */
        public flatMarkdown: string;

        /**
         * Creates a new ExportTreeMarkdownResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ExportTreeMarkdownResponse instance
         */
        public static create(properties?: petajournal.IExportTreeMarkdownResponse): petajournal.ExportTreeMarkdownResponse;

        /**
         * Encodes the specified ExportTreeMarkdownResponse message. Does not implicitly {@link petajournal.ExportTreeMarkdownResponse.verify|verify} messages.
         * @param message ExportTreeMarkdownResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IExportTreeMarkdownResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ExportTreeMarkdownResponse message, length delimited. Does not implicitly {@link petajournal.ExportTreeMarkdownResponse.verify|verify} messages.
         * @param message ExportTreeMarkdownResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IExportTreeMarkdownResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ExportTreeMarkdownResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ExportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ExportTreeMarkdownResponse;

        /**
         * Decodes an ExportTreeMarkdownResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ExportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ExportTreeMarkdownResponse;

        /**
         * Verifies an ExportTreeMarkdownResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ExportTreeMarkdownResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ExportTreeMarkdownResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ExportTreeMarkdownResponse;

        /**
         * Creates a plain object from an ExportTreeMarkdownResponse message. Also converts values to other types if specified.
         * @param message ExportTreeMarkdownResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ExportTreeMarkdownResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ExportTreeMarkdownResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ExportTreeMarkdownResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ImportTreeMarkdownRequest. */
    interface IImportTreeMarkdownRequest {

        /** ImportTreeMarkdownRequest treeMarkdown */
        treeMarkdown?: (string|null);
    }

    /** Represents an ImportTreeMarkdownRequest. */
    class ImportTreeMarkdownRequest implements IImportTreeMarkdownRequest {

        /**
         * Constructs a new ImportTreeMarkdownRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IImportTreeMarkdownRequest);

        /** ImportTreeMarkdownRequest treeMarkdown. */
        public treeMarkdown: string;

        /**
         * Creates a new ImportTreeMarkdownRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ImportTreeMarkdownRequest instance
         */
        public static create(properties?: petajournal.IImportTreeMarkdownRequest): petajournal.ImportTreeMarkdownRequest;

        /**
         * Encodes the specified ImportTreeMarkdownRequest message. Does not implicitly {@link petajournal.ImportTreeMarkdownRequest.verify|verify} messages.
         * @param message ImportTreeMarkdownRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IImportTreeMarkdownRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ImportTreeMarkdownRequest message, length delimited. Does not implicitly {@link petajournal.ImportTreeMarkdownRequest.verify|verify} messages.
         * @param message ImportTreeMarkdownRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IImportTreeMarkdownRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ImportTreeMarkdownRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ImportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ImportTreeMarkdownRequest;

        /**
         * Decodes an ImportTreeMarkdownRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ImportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ImportTreeMarkdownRequest;

        /**
         * Verifies an ImportTreeMarkdownRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ImportTreeMarkdownRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ImportTreeMarkdownRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ImportTreeMarkdownRequest;

        /**
         * Creates a plain object from an ImportTreeMarkdownRequest message. Also converts values to other types if specified.
         * @param message ImportTreeMarkdownRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ImportTreeMarkdownRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ImportTreeMarkdownRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ImportTreeMarkdownRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ImportTreeMarkdownResponse. */
    interface IImportTreeMarkdownResponse {

        /** ImportTreeMarkdownResponse flatMarkdown */
        flatMarkdown?: (string|null);
    }

    /** Represents an ImportTreeMarkdownResponse. */
    class ImportTreeMarkdownResponse implements IImportTreeMarkdownResponse {

        /**
         * Constructs a new ImportTreeMarkdownResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IImportTreeMarkdownResponse);

        /** ImportTreeMarkdownResponse flatMarkdown. */
        public flatMarkdown: string;

        /**
         * Creates a new ImportTreeMarkdownResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ImportTreeMarkdownResponse instance
         */
        public static create(properties?: petajournal.IImportTreeMarkdownResponse): petajournal.ImportTreeMarkdownResponse;

        /**
         * Encodes the specified ImportTreeMarkdownResponse message. Does not implicitly {@link petajournal.ImportTreeMarkdownResponse.verify|verify} messages.
         * @param message ImportTreeMarkdownResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IImportTreeMarkdownResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ImportTreeMarkdownResponse message, length delimited. Does not implicitly {@link petajournal.ImportTreeMarkdownResponse.verify|verify} messages.
         * @param message ImportTreeMarkdownResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IImportTreeMarkdownResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ImportTreeMarkdownResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ImportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ImportTreeMarkdownResponse;

        /**
         * Decodes an ImportTreeMarkdownResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ImportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ImportTreeMarkdownResponse;

        /**
         * Verifies an ImportTreeMarkdownResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ImportTreeMarkdownResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ImportTreeMarkdownResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ImportTreeMarkdownResponse;

        /**
         * Creates a plain object from an ImportTreeMarkdownResponse message. Also converts values to other types if specified.
         * @param message ImportTreeMarkdownResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ImportTreeMarkdownResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ImportTreeMarkdownResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ImportTreeMarkdownResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetBookmarksRequest. */
    interface IGetBookmarksRequest {
    }

    /** Represents a GetBookmarksRequest. */
    class GetBookmarksRequest implements IGetBookmarksRequest {

        /**
         * Constructs a new GetBookmarksRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetBookmarksRequest);

        /**
         * Creates a new GetBookmarksRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetBookmarksRequest instance
         */
        public static create(properties?: petajournal.IGetBookmarksRequest): petajournal.GetBookmarksRequest;

        /**
         * Encodes the specified GetBookmarksRequest message. Does not implicitly {@link petajournal.GetBookmarksRequest.verify|verify} messages.
         * @param message GetBookmarksRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetBookmarksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetBookmarksRequest message, length delimited. Does not implicitly {@link petajournal.GetBookmarksRequest.verify|verify} messages.
         * @param message GetBookmarksRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetBookmarksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetBookmarksRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBookmarksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBookmarksRequest;

        /**
         * Decodes a GetBookmarksRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetBookmarksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetBookmarksRequest;

        /**
         * Verifies a GetBookmarksRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetBookmarksRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetBookmarksRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetBookmarksRequest;

        /**
         * Creates a plain object from a GetBookmarksRequest message. Also converts values to other types if specified.
         * @param message GetBookmarksRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetBookmarksRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetBookmarksRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetBookmarksRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetBookmarksResponse. */
    interface IGetBookmarksResponse {

        /** GetBookmarksResponse groups */
        groups?: (petajournal.IBookmarkGroup[]|null);

        /** GetBookmarksResponse bookmarks */
        bookmarks?: (petajournal.IBookmark[]|null);
    }

    /** Represents a GetBookmarksResponse. */
    class GetBookmarksResponse implements IGetBookmarksResponse {

        /**
         * Constructs a new GetBookmarksResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetBookmarksResponse);

        /** GetBookmarksResponse groups. */
        public groups: petajournal.IBookmarkGroup[];

        /** GetBookmarksResponse bookmarks. */
        public bookmarks: petajournal.IBookmark[];

        /**
         * Creates a new GetBookmarksResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetBookmarksResponse instance
         */
        public static create(properties?: petajournal.IGetBookmarksResponse): petajournal.GetBookmarksResponse;

        /**
         * Encodes the specified GetBookmarksResponse message. Does not implicitly {@link petajournal.GetBookmarksResponse.verify|verify} messages.
         * @param message GetBookmarksResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetBookmarksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetBookmarksResponse message, length delimited. Does not implicitly {@link petajournal.GetBookmarksResponse.verify|verify} messages.
         * @param message GetBookmarksResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IGetBookmarksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetBookmarksResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBookmarksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBookmarksResponse;

        /**
         * Decodes a GetBookmarksResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetBookmarksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.GetBookmarksResponse;

        /**
         * Verifies a GetBookmarksResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetBookmarksResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetBookmarksResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.GetBookmarksResponse;

        /**
         * Creates a plain object from a GetBookmarksResponse message. Also converts values to other types if specified.
         * @param message GetBookmarksResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.GetBookmarksResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetBookmarksResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GetBookmarksResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreateBookmarkGroupRequest. */
    interface ICreateBookmarkGroupRequest {

        /** CreateBookmarkGroupRequest name */
        name?: (string|null);
    }

    /** Represents a CreateBookmarkGroupRequest. */
    class CreateBookmarkGroupRequest implements ICreateBookmarkGroupRequest {

        /**
         * Constructs a new CreateBookmarkGroupRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreateBookmarkGroupRequest);

        /** CreateBookmarkGroupRequest name. */
        public name: string;

        /**
         * Creates a new CreateBookmarkGroupRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreateBookmarkGroupRequest instance
         */
        public static create(properties?: petajournal.ICreateBookmarkGroupRequest): petajournal.CreateBookmarkGroupRequest;

        /**
         * Encodes the specified CreateBookmarkGroupRequest message. Does not implicitly {@link petajournal.CreateBookmarkGroupRequest.verify|verify} messages.
         * @param message CreateBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreateBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CreateBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkGroupRequest.verify|verify} messages.
         * @param message CreateBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ICreateBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreateBookmarkGroupRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBookmarkGroupRequest;

        /**
         * Decodes a CreateBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CreateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.CreateBookmarkGroupRequest;

        /**
         * Verifies a CreateBookmarkGroupRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CreateBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CreateBookmarkGroupRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.CreateBookmarkGroupRequest;

        /**
         * Creates a plain object from a CreateBookmarkGroupRequest message. Also converts values to other types if specified.
         * @param message CreateBookmarkGroupRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.CreateBookmarkGroupRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CreateBookmarkGroupRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for CreateBookmarkGroupRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreateBookmarkGroupResponse. */
    interface ICreateBookmarkGroupResponse {

        /** CreateBookmarkGroupResponse group */
        group?: (petajournal.IBookmarkGroup|null);
    }

    /** Represents a CreateBookmarkGroupResponse. */
    class CreateBookmarkGroupResponse implements ICreateBookmarkGroupResponse {

        /**
         * Constructs a new CreateBookmarkGroupResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreateBookmarkGroupResponse);

        /** CreateBookmarkGroupResponse group. */
        public group?: (petajournal.IBookmarkGroup|null);

        /**
         * Creates a new CreateBookmarkGroupResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreateBookmarkGroupResponse instance
         */
        public static create(properties?: petajournal.ICreateBookmarkGroupResponse): petajournal.CreateBookmarkGroupResponse;

        /**
         * Encodes the specified CreateBookmarkGroupResponse message. Does not implicitly {@link petajournal.CreateBookmarkGroupResponse.verify|verify} messages.
         * @param message CreateBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreateBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CreateBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkGroupResponse.verify|verify} messages.
         * @param message CreateBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ICreateBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreateBookmarkGroupResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBookmarkGroupResponse;

        /**
         * Decodes a CreateBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CreateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.CreateBookmarkGroupResponse;

        /**
         * Verifies a CreateBookmarkGroupResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CreateBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CreateBookmarkGroupResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.CreateBookmarkGroupResponse;

        /**
         * Creates a plain object from a CreateBookmarkGroupResponse message. Also converts values to other types if specified.
         * @param message CreateBookmarkGroupResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.CreateBookmarkGroupResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CreateBookmarkGroupResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for CreateBookmarkGroupResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreateBookmarkRequest. */
    interface ICreateBookmarkRequest {

        /** CreateBookmarkRequest groupId */
        groupId?: (string|null);

        /** CreateBookmarkRequest targetType */
        targetType?: (string|null);

        /** CreateBookmarkRequest targetId */
        targetId?: (string|null);
    }

    /** Represents a CreateBookmarkRequest. */
    class CreateBookmarkRequest implements ICreateBookmarkRequest {

        /**
         * Constructs a new CreateBookmarkRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreateBookmarkRequest);

        /** CreateBookmarkRequest groupId. */
        public groupId: string;

        /** CreateBookmarkRequest targetType. */
        public targetType: string;

        /** CreateBookmarkRequest targetId. */
        public targetId: string;

        /**
         * Creates a new CreateBookmarkRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreateBookmarkRequest instance
         */
        public static create(properties?: petajournal.ICreateBookmarkRequest): petajournal.CreateBookmarkRequest;

        /**
         * Encodes the specified CreateBookmarkRequest message. Does not implicitly {@link petajournal.CreateBookmarkRequest.verify|verify} messages.
         * @param message CreateBookmarkRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreateBookmarkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CreateBookmarkRequest message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkRequest.verify|verify} messages.
         * @param message CreateBookmarkRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ICreateBookmarkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreateBookmarkRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBookmarkRequest;

        /**
         * Decodes a CreateBookmarkRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CreateBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.CreateBookmarkRequest;

        /**
         * Verifies a CreateBookmarkRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CreateBookmarkRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CreateBookmarkRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.CreateBookmarkRequest;

        /**
         * Creates a plain object from a CreateBookmarkRequest message. Also converts values to other types if specified.
         * @param message CreateBookmarkRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.CreateBookmarkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CreateBookmarkRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for CreateBookmarkRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreateBookmarkResponse. */
    interface ICreateBookmarkResponse {

        /** CreateBookmarkResponse bookmark */
        bookmark?: (petajournal.IBookmark|null);
    }

    /** Represents a CreateBookmarkResponse. */
    class CreateBookmarkResponse implements ICreateBookmarkResponse {

        /**
         * Constructs a new CreateBookmarkResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreateBookmarkResponse);

        /** CreateBookmarkResponse bookmark. */
        public bookmark?: (petajournal.IBookmark|null);

        /**
         * Creates a new CreateBookmarkResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreateBookmarkResponse instance
         */
        public static create(properties?: petajournal.ICreateBookmarkResponse): petajournal.CreateBookmarkResponse;

        /**
         * Encodes the specified CreateBookmarkResponse message. Does not implicitly {@link petajournal.CreateBookmarkResponse.verify|verify} messages.
         * @param message CreateBookmarkResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreateBookmarkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CreateBookmarkResponse message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkResponse.verify|verify} messages.
         * @param message CreateBookmarkResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.ICreateBookmarkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreateBookmarkResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBookmarkResponse;

        /**
         * Decodes a CreateBookmarkResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CreateBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.CreateBookmarkResponse;

        /**
         * Verifies a CreateBookmarkResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CreateBookmarkResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CreateBookmarkResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.CreateBookmarkResponse;

        /**
         * Creates a plain object from a CreateBookmarkResponse message. Also converts values to other types if specified.
         * @param message CreateBookmarkResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.CreateBookmarkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CreateBookmarkResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for CreateBookmarkResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeleteBookmarkRequest. */
    interface IDeleteBookmarkRequest {

        /** DeleteBookmarkRequest bookmarkId */
        bookmarkId?: (string|null);
    }

    /** Represents a DeleteBookmarkRequest. */
    class DeleteBookmarkRequest implements IDeleteBookmarkRequest {

        /**
         * Constructs a new DeleteBookmarkRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeleteBookmarkRequest);

        /** DeleteBookmarkRequest bookmarkId. */
        public bookmarkId: string;

        /**
         * Creates a new DeleteBookmarkRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeleteBookmarkRequest instance
         */
        public static create(properties?: petajournal.IDeleteBookmarkRequest): petajournal.DeleteBookmarkRequest;

        /**
         * Encodes the specified DeleteBookmarkRequest message. Does not implicitly {@link petajournal.DeleteBookmarkRequest.verify|verify} messages.
         * @param message DeleteBookmarkRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeleteBookmarkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeleteBookmarkRequest message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkRequest.verify|verify} messages.
         * @param message DeleteBookmarkRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeleteBookmarkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeleteBookmarkRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBookmarkRequest;

        /**
         * Decodes a DeleteBookmarkRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeleteBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeleteBookmarkRequest;

        /**
         * Verifies a DeleteBookmarkRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeleteBookmarkRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeleteBookmarkRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeleteBookmarkRequest;

        /**
         * Creates a plain object from a DeleteBookmarkRequest message. Also converts values to other types if specified.
         * @param message DeleteBookmarkRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeleteBookmarkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeleteBookmarkRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeleteBookmarkRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeleteBookmarkResponse. */
    interface IDeleteBookmarkResponse {
    }

    /** Represents a DeleteBookmarkResponse. */
    class DeleteBookmarkResponse implements IDeleteBookmarkResponse {

        /**
         * Constructs a new DeleteBookmarkResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeleteBookmarkResponse);

        /**
         * Creates a new DeleteBookmarkResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeleteBookmarkResponse instance
         */
        public static create(properties?: petajournal.IDeleteBookmarkResponse): petajournal.DeleteBookmarkResponse;

        /**
         * Encodes the specified DeleteBookmarkResponse message. Does not implicitly {@link petajournal.DeleteBookmarkResponse.verify|verify} messages.
         * @param message DeleteBookmarkResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeleteBookmarkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeleteBookmarkResponse message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkResponse.verify|verify} messages.
         * @param message DeleteBookmarkResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeleteBookmarkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeleteBookmarkResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBookmarkResponse;

        /**
         * Decodes a DeleteBookmarkResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeleteBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeleteBookmarkResponse;

        /**
         * Verifies a DeleteBookmarkResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeleteBookmarkResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeleteBookmarkResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeleteBookmarkResponse;

        /**
         * Creates a plain object from a DeleteBookmarkResponse message. Also converts values to other types if specified.
         * @param message DeleteBookmarkResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeleteBookmarkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeleteBookmarkResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeleteBookmarkResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeleteBookmarkGroupRequest. */
    interface IDeleteBookmarkGroupRequest {

        /** DeleteBookmarkGroupRequest groupId */
        groupId?: (string|null);
    }

    /** Represents a DeleteBookmarkGroupRequest. */
    class DeleteBookmarkGroupRequest implements IDeleteBookmarkGroupRequest {

        /**
         * Constructs a new DeleteBookmarkGroupRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeleteBookmarkGroupRequest);

        /** DeleteBookmarkGroupRequest groupId. */
        public groupId: string;

        /**
         * Creates a new DeleteBookmarkGroupRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeleteBookmarkGroupRequest instance
         */
        public static create(properties?: petajournal.IDeleteBookmarkGroupRequest): petajournal.DeleteBookmarkGroupRequest;

        /**
         * Encodes the specified DeleteBookmarkGroupRequest message. Does not implicitly {@link petajournal.DeleteBookmarkGroupRequest.verify|verify} messages.
         * @param message DeleteBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeleteBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeleteBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkGroupRequest.verify|verify} messages.
         * @param message DeleteBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeleteBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeleteBookmarkGroupRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBookmarkGroupRequest;

        /**
         * Decodes a DeleteBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeleteBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeleteBookmarkGroupRequest;

        /**
         * Verifies a DeleteBookmarkGroupRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeleteBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeleteBookmarkGroupRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeleteBookmarkGroupRequest;

        /**
         * Creates a plain object from a DeleteBookmarkGroupRequest message. Also converts values to other types if specified.
         * @param message DeleteBookmarkGroupRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeleteBookmarkGroupRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeleteBookmarkGroupRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeleteBookmarkGroupRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DeleteBookmarkGroupResponse. */
    interface IDeleteBookmarkGroupResponse {
    }

    /** Represents a DeleteBookmarkGroupResponse. */
    class DeleteBookmarkGroupResponse implements IDeleteBookmarkGroupResponse {

        /**
         * Constructs a new DeleteBookmarkGroupResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IDeleteBookmarkGroupResponse);

        /**
         * Creates a new DeleteBookmarkGroupResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DeleteBookmarkGroupResponse instance
         */
        public static create(properties?: petajournal.IDeleteBookmarkGroupResponse): petajournal.DeleteBookmarkGroupResponse;

        /**
         * Encodes the specified DeleteBookmarkGroupResponse message. Does not implicitly {@link petajournal.DeleteBookmarkGroupResponse.verify|verify} messages.
         * @param message DeleteBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IDeleteBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DeleteBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkGroupResponse.verify|verify} messages.
         * @param message DeleteBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IDeleteBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DeleteBookmarkGroupResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBookmarkGroupResponse;

        /**
         * Decodes a DeleteBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DeleteBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.DeleteBookmarkGroupResponse;

        /**
         * Verifies a DeleteBookmarkGroupResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DeleteBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DeleteBookmarkGroupResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.DeleteBookmarkGroupResponse;

        /**
         * Creates a plain object from a DeleteBookmarkGroupResponse message. Also converts values to other types if specified.
         * @param message DeleteBookmarkGroupResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.DeleteBookmarkGroupResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DeleteBookmarkGroupResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DeleteBookmarkGroupResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MoveBookmarkRequest. */
    interface IMoveBookmarkRequest {

        /** MoveBookmarkRequest bookmarkId */
        bookmarkId?: (string|null);

        /** MoveBookmarkRequest targetGroupId */
        targetGroupId?: (string|null);

        /** MoveBookmarkRequest afterBookmarkId */
        afterBookmarkId?: (string|null);
    }

    /** Represents a MoveBookmarkRequest. */
    class MoveBookmarkRequest implements IMoveBookmarkRequest {

        /**
         * Constructs a new MoveBookmarkRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IMoveBookmarkRequest);

        /** MoveBookmarkRequest bookmarkId. */
        public bookmarkId: string;

        /** MoveBookmarkRequest targetGroupId. */
        public targetGroupId: string;

        /** MoveBookmarkRequest afterBookmarkId. */
        public afterBookmarkId: string;

        /**
         * Creates a new MoveBookmarkRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MoveBookmarkRequest instance
         */
        public static create(properties?: petajournal.IMoveBookmarkRequest): petajournal.MoveBookmarkRequest;

        /**
         * Encodes the specified MoveBookmarkRequest message. Does not implicitly {@link petajournal.MoveBookmarkRequest.verify|verify} messages.
         * @param message MoveBookmarkRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IMoveBookmarkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MoveBookmarkRequest message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkRequest.verify|verify} messages.
         * @param message MoveBookmarkRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IMoveBookmarkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MoveBookmarkRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBookmarkRequest;

        /**
         * Decodes a MoveBookmarkRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MoveBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.MoveBookmarkRequest;

        /**
         * Verifies a MoveBookmarkRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MoveBookmarkRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MoveBookmarkRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.MoveBookmarkRequest;

        /**
         * Creates a plain object from a MoveBookmarkRequest message. Also converts values to other types if specified.
         * @param message MoveBookmarkRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.MoveBookmarkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MoveBookmarkRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MoveBookmarkRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MoveBookmarkResponse. */
    interface IMoveBookmarkResponse {

        /** MoveBookmarkResponse bookmark */
        bookmark?: (petajournal.IBookmark|null);
    }

    /** Represents a MoveBookmarkResponse. */
    class MoveBookmarkResponse implements IMoveBookmarkResponse {

        /**
         * Constructs a new MoveBookmarkResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IMoveBookmarkResponse);

        /** MoveBookmarkResponse bookmark. */
        public bookmark?: (petajournal.IBookmark|null);

        /**
         * Creates a new MoveBookmarkResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MoveBookmarkResponse instance
         */
        public static create(properties?: petajournal.IMoveBookmarkResponse): petajournal.MoveBookmarkResponse;

        /**
         * Encodes the specified MoveBookmarkResponse message. Does not implicitly {@link petajournal.MoveBookmarkResponse.verify|verify} messages.
         * @param message MoveBookmarkResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IMoveBookmarkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MoveBookmarkResponse message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkResponse.verify|verify} messages.
         * @param message MoveBookmarkResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IMoveBookmarkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MoveBookmarkResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBookmarkResponse;

        /**
         * Decodes a MoveBookmarkResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MoveBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.MoveBookmarkResponse;

        /**
         * Verifies a MoveBookmarkResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MoveBookmarkResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MoveBookmarkResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.MoveBookmarkResponse;

        /**
         * Creates a plain object from a MoveBookmarkResponse message. Also converts values to other types if specified.
         * @param message MoveBookmarkResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.MoveBookmarkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MoveBookmarkResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MoveBookmarkResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MoveBookmarkGroupRequest. */
    interface IMoveBookmarkGroupRequest {

        /** MoveBookmarkGroupRequest groupId */
        groupId?: (string|null);

        /** MoveBookmarkGroupRequest afterGroupId */
        afterGroupId?: (string|null);
    }

    /** Represents a MoveBookmarkGroupRequest. */
    class MoveBookmarkGroupRequest implements IMoveBookmarkGroupRequest {

        /**
         * Constructs a new MoveBookmarkGroupRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IMoveBookmarkGroupRequest);

        /** MoveBookmarkGroupRequest groupId. */
        public groupId: string;

        /** MoveBookmarkGroupRequest afterGroupId. */
        public afterGroupId: string;

        /**
         * Creates a new MoveBookmarkGroupRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MoveBookmarkGroupRequest instance
         */
        public static create(properties?: petajournal.IMoveBookmarkGroupRequest): petajournal.MoveBookmarkGroupRequest;

        /**
         * Encodes the specified MoveBookmarkGroupRequest message. Does not implicitly {@link petajournal.MoveBookmarkGroupRequest.verify|verify} messages.
         * @param message MoveBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IMoveBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MoveBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkGroupRequest.verify|verify} messages.
         * @param message MoveBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IMoveBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MoveBookmarkGroupRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBookmarkGroupRequest;

        /**
         * Decodes a MoveBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MoveBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.MoveBookmarkGroupRequest;

        /**
         * Verifies a MoveBookmarkGroupRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MoveBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MoveBookmarkGroupRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.MoveBookmarkGroupRequest;

        /**
         * Creates a plain object from a MoveBookmarkGroupRequest message. Also converts values to other types if specified.
         * @param message MoveBookmarkGroupRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.MoveBookmarkGroupRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MoveBookmarkGroupRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MoveBookmarkGroupRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MoveBookmarkGroupResponse. */
    interface IMoveBookmarkGroupResponse {

        /** MoveBookmarkGroupResponse group */
        group?: (petajournal.IBookmarkGroup|null);
    }

    /** Represents a MoveBookmarkGroupResponse. */
    class MoveBookmarkGroupResponse implements IMoveBookmarkGroupResponse {

        /**
         * Constructs a new MoveBookmarkGroupResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IMoveBookmarkGroupResponse);

        /** MoveBookmarkGroupResponse group. */
        public group?: (petajournal.IBookmarkGroup|null);

        /**
         * Creates a new MoveBookmarkGroupResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MoveBookmarkGroupResponse instance
         */
        public static create(properties?: petajournal.IMoveBookmarkGroupResponse): petajournal.MoveBookmarkGroupResponse;

        /**
         * Encodes the specified MoveBookmarkGroupResponse message. Does not implicitly {@link petajournal.MoveBookmarkGroupResponse.verify|verify} messages.
         * @param message MoveBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IMoveBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MoveBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkGroupResponse.verify|verify} messages.
         * @param message MoveBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IMoveBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MoveBookmarkGroupResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBookmarkGroupResponse;

        /**
         * Decodes a MoveBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MoveBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.MoveBookmarkGroupResponse;

        /**
         * Verifies a MoveBookmarkGroupResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MoveBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MoveBookmarkGroupResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.MoveBookmarkGroupResponse;

        /**
         * Creates a plain object from a MoveBookmarkGroupResponse message. Also converts values to other types if specified.
         * @param message MoveBookmarkGroupResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.MoveBookmarkGroupResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MoveBookmarkGroupResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MoveBookmarkGroupResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateBookmarkGroupRequest. */
    interface IUpdateBookmarkGroupRequest {

        /** UpdateBookmarkGroupRequest groupId */
        groupId?: (string|null);

        /** UpdateBookmarkGroupRequest name */
        name?: (string|null);
    }

    /** Represents an UpdateBookmarkGroupRequest. */
    class UpdateBookmarkGroupRequest implements IUpdateBookmarkGroupRequest {

        /**
         * Constructs a new UpdateBookmarkGroupRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateBookmarkGroupRequest);

        /** UpdateBookmarkGroupRequest groupId. */
        public groupId: string;

        /** UpdateBookmarkGroupRequest name. */
        public name: string;

        /**
         * Creates a new UpdateBookmarkGroupRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateBookmarkGroupRequest instance
         */
        public static create(properties?: petajournal.IUpdateBookmarkGroupRequest): petajournal.UpdateBookmarkGroupRequest;

        /**
         * Encodes the specified UpdateBookmarkGroupRequest message. Does not implicitly {@link petajournal.UpdateBookmarkGroupRequest.verify|verify} messages.
         * @param message UpdateBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.UpdateBookmarkGroupRequest.verify|verify} messages.
         * @param message UpdateBookmarkGroupRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateBookmarkGroupRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateBookmarkGroupRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBookmarkGroupRequest;

        /**
         * Decodes an UpdateBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateBookmarkGroupRequest;

        /**
         * Verifies an UpdateBookmarkGroupRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateBookmarkGroupRequest
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateBookmarkGroupRequest;

        /**
         * Creates a plain object from an UpdateBookmarkGroupRequest message. Also converts values to other types if specified.
         * @param message UpdateBookmarkGroupRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateBookmarkGroupRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateBookmarkGroupRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateBookmarkGroupRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an UpdateBookmarkGroupResponse. */
    interface IUpdateBookmarkGroupResponse {

        /** UpdateBookmarkGroupResponse group */
        group?: (petajournal.IBookmarkGroup|null);
    }

    /** Represents an UpdateBookmarkGroupResponse. */
    class UpdateBookmarkGroupResponse implements IUpdateBookmarkGroupResponse {

        /**
         * Constructs a new UpdateBookmarkGroupResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IUpdateBookmarkGroupResponse);

        /** UpdateBookmarkGroupResponse group. */
        public group?: (petajournal.IBookmarkGroup|null);

        /**
         * Creates a new UpdateBookmarkGroupResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns UpdateBookmarkGroupResponse instance
         */
        public static create(properties?: petajournal.IUpdateBookmarkGroupResponse): petajournal.UpdateBookmarkGroupResponse;

        /**
         * Encodes the specified UpdateBookmarkGroupResponse message. Does not implicitly {@link petajournal.UpdateBookmarkGroupResponse.verify|verify} messages.
         * @param message UpdateBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IUpdateBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified UpdateBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.UpdateBookmarkGroupResponse.verify|verify} messages.
         * @param message UpdateBookmarkGroupResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IUpdateBookmarkGroupResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an UpdateBookmarkGroupResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBookmarkGroupResponse;

        /**
         * Decodes an UpdateBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns UpdateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.UpdateBookmarkGroupResponse;

        /**
         * Verifies an UpdateBookmarkGroupResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an UpdateBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns UpdateBookmarkGroupResponse
         */
        public static fromObject(object: { [k: string]: any }): petajournal.UpdateBookmarkGroupResponse;

        /**
         * Creates a plain object from an UpdateBookmarkGroupResponse message. Also converts values to other types if specified.
         * @param message UpdateBookmarkGroupResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.UpdateBookmarkGroupResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this UpdateBookmarkGroupResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for UpdateBookmarkGroupResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ServerEvent. */
    interface IServerEvent {

        /** ServerEvent originatorSessionId */
        originatorSessionId?: (string|null);

        /** ServerEvent connected */
        connected?: (petajournal.IConnectedEvent|null);

        /** ServerEvent pagesUpdated */
        pagesUpdated?: (petajournal.IPagesUpdatedEvent|null);
    }

    /** Represents a ServerEvent. */
    class ServerEvent implements IServerEvent {

        /**
         * Constructs a new ServerEvent.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IServerEvent);

        /** ServerEvent originatorSessionId. */
        public originatorSessionId: string;

        /** ServerEvent connected. */
        public connected?: (petajournal.IConnectedEvent|null);

        /** ServerEvent pagesUpdated. */
        public pagesUpdated?: (petajournal.IPagesUpdatedEvent|null);

        /** ServerEvent event. */
        public event?: ("connected"|"pagesUpdated");

        /**
         * Creates a new ServerEvent instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ServerEvent instance
         */
        public static create(properties?: petajournal.IServerEvent): petajournal.ServerEvent;

        /**
         * Encodes the specified ServerEvent message. Does not implicitly {@link petajournal.ServerEvent.verify|verify} messages.
         * @param message ServerEvent message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IServerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ServerEvent message, length delimited. Does not implicitly {@link petajournal.ServerEvent.verify|verify} messages.
         * @param message ServerEvent message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IServerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ServerEvent message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ServerEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ServerEvent;

        /**
         * Decodes a ServerEvent message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ServerEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ServerEvent;

        /**
         * Verifies a ServerEvent message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ServerEvent message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ServerEvent
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ServerEvent;

        /**
         * Creates a plain object from a ServerEvent message. Also converts values to other types if specified.
         * @param message ServerEvent
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ServerEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ServerEvent to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ServerEvent
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectedEvent. */
    interface IConnectedEvent {

        /** ConnectedEvent sessionId */
        sessionId?: (string|null);
    }

    /** Represents a ConnectedEvent. */
    class ConnectedEvent implements IConnectedEvent {

        /**
         * Constructs a new ConnectedEvent.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IConnectedEvent);

        /** ConnectedEvent sessionId. */
        public sessionId: string;

        /**
         * Creates a new ConnectedEvent instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectedEvent instance
         */
        public static create(properties?: petajournal.IConnectedEvent): petajournal.ConnectedEvent;

        /**
         * Encodes the specified ConnectedEvent message. Does not implicitly {@link petajournal.ConnectedEvent.verify|verify} messages.
         * @param message ConnectedEvent message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IConnectedEvent, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectedEvent message, length delimited. Does not implicitly {@link petajournal.ConnectedEvent.verify|verify} messages.
         * @param message ConnectedEvent message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IConnectedEvent, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectedEvent message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ConnectedEvent;

        /**
         * Decodes a ConnectedEvent message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.ConnectedEvent;

        /**
         * Verifies a ConnectedEvent message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectedEvent message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectedEvent
         */
        public static fromObject(object: { [k: string]: any }): petajournal.ConnectedEvent;

        /**
         * Creates a plain object from a ConnectedEvent message. Also converts values to other types if specified.
         * @param message ConnectedEvent
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.ConnectedEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectedEvent to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectedEvent
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a PagesUpdatedEvent. */
    interface IPagesUpdatedEvent {

        /** PagesUpdatedEvent pageId */
        pageId?: (string|null);
    }

    /** Represents a PagesUpdatedEvent. */
    class PagesUpdatedEvent implements IPagesUpdatedEvent {

        /**
         * Constructs a new PagesUpdatedEvent.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IPagesUpdatedEvent);

        /** PagesUpdatedEvent pageId. */
        public pageId: string;

        /**
         * Creates a new PagesUpdatedEvent instance using the specified properties.
         * @param [properties] Properties to set
         * @returns PagesUpdatedEvent instance
         */
        public static create(properties?: petajournal.IPagesUpdatedEvent): petajournal.PagesUpdatedEvent;

        /**
         * Encodes the specified PagesUpdatedEvent message. Does not implicitly {@link petajournal.PagesUpdatedEvent.verify|verify} messages.
         * @param message PagesUpdatedEvent message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IPagesUpdatedEvent, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified PagesUpdatedEvent message, length delimited. Does not implicitly {@link petajournal.PagesUpdatedEvent.verify|verify} messages.
         * @param message PagesUpdatedEvent message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: petajournal.IPagesUpdatedEvent, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a PagesUpdatedEvent message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns PagesUpdatedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.PagesUpdatedEvent;

        /**
         * Decodes a PagesUpdatedEvent message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns PagesUpdatedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): petajournal.PagesUpdatedEvent;

        /**
         * Verifies a PagesUpdatedEvent message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a PagesUpdatedEvent message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns PagesUpdatedEvent
         */
        public static fromObject(object: { [k: string]: any }): petajournal.PagesUpdatedEvent;

        /**
         * Creates a plain object from a PagesUpdatedEvent message. Also converts values to other types if specified.
         * @param message PagesUpdatedEvent
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: petajournal.PagesUpdatedEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this PagesUpdatedEvent to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for PagesUpdatedEvent
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }
}
