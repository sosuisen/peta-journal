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
         * Decodes a Timestamp message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Timestamp
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Timestamp;

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

        /** Page fileName */
        fileName?: (string|null);

        /** Page createdDate */
        createdDate?: (petajournal.ITimestamp|null);

        /** Page modifiedDate */
        modifiedDate?: (petajournal.ITimestamp|null);

        /** Page tags */
        tags?: (string[]|null);

        /** Page propertiesJson */
        propertiesJson?: (string|null);
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

        /** Page fileName. */
        public fileName: string;

        /** Page createdDate. */
        public createdDate?: (petajournal.ITimestamp|null);

        /** Page modifiedDate. */
        public modifiedDate?: (petajournal.ITimestamp|null);

        /** Page tags. */
        public tags: string[];

        /** Page propertiesJson. */
        public propertiesJson: string;

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
         * Decodes a Page message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Page
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Page;

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

        /** Block fileName */
        fileName?: (string|null);

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

        /** Block propertiesJson */
        propertiesJson?: (string|null);
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

        /** Block fileName. */
        public fileName: string;

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

        /** Block propertiesJson. */
        public propertiesJson: string;

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
         * Decodes a Block message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Block
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.Block;

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
         * Decodes a SearchRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchRequest;

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
         * Decodes a SearchResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchResponse;

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
         * Decodes a SearchResult message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchResult;

        /**
         * Gets the default type url for SearchResult
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ApiRequest. */
    interface IApiRequest {

        /** ApiRequest requestId */
        requestId?: (string|null);

        /** ApiRequest getPage */
        getPage?: (petajournal.IGetPageRequest|null);

        /** ApiRequest getJournal */
        getJournal?: (petajournal.IGetJournalRequest|null);

        /** ApiRequest createPage */
        createPage?: (petajournal.ICreatePageRequest|null);

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

        /** ApiRequest getPage. */
        public getPage?: (petajournal.IGetPageRequest|null);

        /** ApiRequest getJournal. */
        public getJournal?: (petajournal.IGetJournalRequest|null);

        /** ApiRequest createPage. */
        public createPage?: (petajournal.ICreatePageRequest|null);

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

        /** ApiRequest payload. */
        public payload?: ("getPage"|"getJournal"|"createPage"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getBlocksByStatus"|"getConfig"|"updateConfig");

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
         * Decodes an ApiRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ApiRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ApiRequest;

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

        /** ApiResponse getPage */
        getPage?: (petajournal.IGetPageResponse|null);

        /** ApiResponse getJournal */
        getJournal?: (petajournal.IGetJournalResponse|null);

        /** ApiResponse createPage */
        createPage?: (petajournal.ICreatePageResponse|null);

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

        /** ApiResponse getBlocksByStatus */
        getBlocksByStatus?: (petajournal.IGetBlocksByStatusResponse|null);

        /** ApiResponse getConfig */
        getConfig?: (petajournal.IGetConfigResponse|null);

        /** ApiResponse updateConfig */
        updateConfig?: (petajournal.IUpdateConfigResponse|null);
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

        /** ApiResponse getPage. */
        public getPage?: (petajournal.IGetPageResponse|null);

        /** ApiResponse getJournal. */
        public getJournal?: (petajournal.IGetJournalResponse|null);

        /** ApiResponse createPage. */
        public createPage?: (petajournal.ICreatePageResponse|null);

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

        /** ApiResponse getBlocksByStatus. */
        public getBlocksByStatus?: (petajournal.IGetBlocksByStatusResponse|null);

        /** ApiResponse getConfig. */
        public getConfig?: (petajournal.IGetConfigResponse|null);

        /** ApiResponse updateConfig. */
        public updateConfig?: (petajournal.IUpdateConfigResponse|null);

        /** ApiResponse payload. */
        public payload?: ("getPage"|"getJournal"|"createPage"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getBlocksByStatus"|"getConfig"|"updateConfig");

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
         * Decodes an ApiResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ApiResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.ApiResponse;

        /**
         * Gets the default type url for ApiResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetPageRequest. */
    interface IGetPageRequest {

        /** GetPageRequest pageId */
        pageId?: (string|null);
    }

    /** Represents a GetPageRequest. */
    class GetPageRequest implements IGetPageRequest {

        /**
         * Constructs a new GetPageRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetPageRequest);

        /** GetPageRequest pageId. */
        public pageId: string;

        /**
         * Creates a new GetPageRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetPageRequest instance
         */
        public static create(properties?: petajournal.IGetPageRequest): petajournal.GetPageRequest;

        /**
         * Encodes the specified GetPageRequest message. Does not implicitly {@link petajournal.GetPageRequest.verify|verify} messages.
         * @param message GetPageRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetPageRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetPageRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetPageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetPageRequest;

        /**
         * Gets the default type url for GetPageRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetPageResponse. */
    interface IGetPageResponse {

        /** GetPageResponse page */
        page?: (petajournal.IPage|null);

        /** GetPageResponse blocks */
        blocks?: (petajournal.IBlock[]|null);
    }

    /** Represents a GetPageResponse. */
    class GetPageResponse implements IGetPageResponse {

        /**
         * Constructs a new GetPageResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetPageResponse);

        /** GetPageResponse page. */
        public page?: (petajournal.IPage|null);

        /** GetPageResponse blocks. */
        public blocks: petajournal.IBlock[];

        /**
         * Creates a new GetPageResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetPageResponse instance
         */
        public static create(properties?: petajournal.IGetPageResponse): petajournal.GetPageResponse;

        /**
         * Encodes the specified GetPageResponse message. Does not implicitly {@link petajournal.GetPageResponse.verify|verify} messages.
         * @param message GetPageResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetPageResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetPageResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetPageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetPageResponse;

        /**
         * Gets the default type url for GetPageResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetJournalRequest. */
    interface IGetJournalRequest {

        /** GetJournalRequest date */
        date?: (string|null);
    }

    /** Represents a GetJournalRequest. */
    class GetJournalRequest implements IGetJournalRequest {

        /**
         * Constructs a new GetJournalRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetJournalRequest);

        /** GetJournalRequest date. */
        public date: string;

        /**
         * Creates a new GetJournalRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetJournalRequest instance
         */
        public static create(properties?: petajournal.IGetJournalRequest): petajournal.GetJournalRequest;

        /**
         * Encodes the specified GetJournalRequest message. Does not implicitly {@link petajournal.GetJournalRequest.verify|verify} messages.
         * @param message GetJournalRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetJournalRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetJournalRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetJournalRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetJournalRequest;

        /**
         * Gets the default type url for GetJournalRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GetJournalResponse. */
    interface IGetJournalResponse {

        /** GetJournalResponse page */
        page?: (petajournal.IPage|null);

        /** GetJournalResponse blocks */
        blocks?: (petajournal.IBlock[]|null);

        /** GetJournalResponse created */
        created?: (boolean|null);
    }

    /** Represents a GetJournalResponse. */
    class GetJournalResponse implements IGetJournalResponse {

        /**
         * Constructs a new GetJournalResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.IGetJournalResponse);

        /** GetJournalResponse page. */
        public page?: (petajournal.IPage|null);

        /** GetJournalResponse blocks. */
        public blocks: petajournal.IBlock[];

        /** GetJournalResponse created. */
        public created: boolean;

        /**
         * Creates a new GetJournalResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetJournalResponse instance
         */
        public static create(properties?: petajournal.IGetJournalResponse): petajournal.GetJournalResponse;

        /**
         * Encodes the specified GetJournalResponse message. Does not implicitly {@link petajournal.GetJournalResponse.verify|verify} messages.
         * @param message GetJournalResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.IGetJournalResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetJournalResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetJournalResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetJournalResponse;

        /**
         * Gets the default type url for GetJournalResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreatePageRequest. */
    interface ICreatePageRequest {

        /** CreatePageRequest title */
        title?: (string|null);

        /** CreatePageRequest dirPath */
        dirPath?: (string|null);

        /** CreatePageRequest tags */
        tags?: (string[]|null);
    }

    /** Represents a CreatePageRequest. */
    class CreatePageRequest implements ICreatePageRequest {

        /**
         * Constructs a new CreatePageRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreatePageRequest);

        /** CreatePageRequest title. */
        public title: string;

        /** CreatePageRequest dirPath. */
        public dirPath: string;

        /** CreatePageRequest tags. */
        public tags: string[];

        /**
         * Creates a new CreatePageRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreatePageRequest instance
         */
        public static create(properties?: petajournal.ICreatePageRequest): petajournal.CreatePageRequest;

        /**
         * Encodes the specified CreatePageRequest message. Does not implicitly {@link petajournal.CreatePageRequest.verify|verify} messages.
         * @param message CreatePageRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreatePageRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreatePageRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreatePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreatePageRequest;

        /**
         * Gets the default type url for CreatePageRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a CreatePageResponse. */
    interface ICreatePageResponse {

        /** CreatePageResponse page */
        page?: (petajournal.IPage|null);
    }

    /** Represents a CreatePageResponse. */
    class CreatePageResponse implements ICreatePageResponse {

        /**
         * Constructs a new CreatePageResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: petajournal.ICreatePageResponse);

        /** CreatePageResponse page. */
        public page?: (petajournal.IPage|null);

        /**
         * Creates a new CreatePageResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CreatePageResponse instance
         */
        public static create(properties?: petajournal.ICreatePageResponse): petajournal.CreatePageResponse;

        /**
         * Encodes the specified CreatePageResponse message. Does not implicitly {@link petajournal.CreatePageResponse.verify|verify} messages.
         * @param message CreatePageResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: petajournal.ICreatePageResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CreatePageResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreatePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreatePageResponse;

        /**
         * Gets the default type url for CreatePageResponse
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
         * Decodes a DeletePageRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeletePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeletePageRequest;

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
         * Decodes a DeletePageResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeletePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeletePageResponse;

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

        /** UpdatePageRequest propertiesJson */
        propertiesJson?: (string|null);
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

        /** UpdatePageRequest propertiesJson. */
        public propertiesJson: string;

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
         * Decodes an UpdatePageRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdatePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdatePageRequest;

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
         * Decodes an UpdatePageResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdatePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdatePageResponse;

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
         * Decodes a CreateBlockRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBlockRequest;

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
         * Decodes a CreateBlockResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CreateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.CreateBlockResponse;

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

        /** UpdateBlockRequest propertiesJson */
        propertiesJson?: (string|null);
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

        /** UpdateBlockRequest propertiesJson. */
        public propertiesJson?: (string|null);

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
         * Decodes an UpdateBlockRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockRequest;

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
         * Decodes an UpdateBlockResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockResponse;

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
         * Decodes a MoveBlocksRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBlocksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBlocksRequest;

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
         * Decodes a MoveBlocksResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MoveBlocksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.MoveBlocksResponse;

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
         * Decodes a DeleteBlockRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBlockRequest;

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
         * Decodes a DeleteBlockResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DeleteBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.DeleteBlockResponse;

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
         * Decodes an UpdateBlockStatusRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockStatusRequest;

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
         * Decodes an UpdateBlockStatusResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateBlockStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateBlockStatusResponse;

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
         * Decodes a SearchPageNamesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchPageNamesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchPageNamesRequest;

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
         * Decodes a SearchPageNamesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchPageNamesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchPageNamesResponse;

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
         * Decodes a SearchFullTextRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchFullTextRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchFullTextRequest;

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
         * Decodes a SearchFullTextResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchFullTextResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.SearchFullTextResponse;

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
         * Decodes a GetRecentPagesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetRecentPagesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetRecentPagesRequest;

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
         * Decodes a GetRecentPagesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetRecentPagesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetRecentPagesResponse;

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
         * Decodes a GetBacklinksRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBacklinksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBacklinksRequest;

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
         * Decodes a GetBacklinksResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBacklinksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBacklinksResponse;

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
         * Decodes a BacklinkEntry message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns BacklinkEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.BacklinkEntry;

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
         * Decodes a GetMostRecentJournalBeforeRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetMostRecentJournalBeforeRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetMostRecentJournalBeforeRequest;

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
         * Decodes a GetMostRecentJournalBeforeResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetMostRecentJournalBeforeResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetMostRecentJournalBeforeResponse;

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
         * Decodes a GetBlocksByStatusRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBlocksByStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBlocksByStatusRequest;

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
         * Decodes a GetBlocksByStatusResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetBlocksByStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetBlocksByStatusResponse;

        /**
         * Gets the default type url for GetBlocksByStatusResponse
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
         * Decodes a GetConfigRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetConfigRequest;

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
         * Decodes a GetConfigResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.GetConfigResponse;

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
         * Decodes an UpdateConfigRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateConfigRequest;

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
         * Decodes an UpdateConfigResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns UpdateConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): petajournal.UpdateConfigResponse;

        /**
         * Gets the default type url for UpdateConfigResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }
}
