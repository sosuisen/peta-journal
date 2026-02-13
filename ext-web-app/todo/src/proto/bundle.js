/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
import * as $protobuf from "protobufjs/minimal";

// Common aliases
const $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;

// Exported root namespace
const $root = $protobuf.roots["default"] || ($protobuf.roots["default"] = {});

export const petajournal = $root.petajournal = (() => {

    /**
     * Namespace petajournal.
     * @exports petajournal
     * @namespace
     */
    const petajournal = {};

    petajournal.Timestamp = (function() {

        /**
         * Properties of a Timestamp.
         * @memberof petajournal
         * @interface ITimestamp
         * @property {number|Long|null} [millis] Timestamp millis
         */

        /**
         * Constructs a new Timestamp.
         * @memberof petajournal
         * @classdesc Represents a Timestamp.
         * @implements ITimestamp
         * @constructor
         * @param {petajournal.ITimestamp=} [properties] Properties to set
         */
        function Timestamp(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Timestamp millis.
         * @member {number|Long} millis
         * @memberof petajournal.Timestamp
         * @instance
         */
        Timestamp.prototype.millis = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

        /**
         * Creates a new Timestamp instance using the specified properties.
         * @function create
         * @memberof petajournal.Timestamp
         * @static
         * @param {petajournal.ITimestamp=} [properties] Properties to set
         * @returns {petajournal.Timestamp} Timestamp instance
         */
        Timestamp.create = function create(properties) {
            return new Timestamp(properties);
        };

        /**
         * Encodes the specified Timestamp message. Does not implicitly {@link petajournal.Timestamp.verify|verify} messages.
         * @function encode
         * @memberof petajournal.Timestamp
         * @static
         * @param {petajournal.ITimestamp} message Timestamp message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Timestamp.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.millis != null && Object.hasOwnProperty.call(message, "millis"))
                writer.uint32(/* id 1, wireType 0 =*/8).int64(message.millis);
            return writer;
        };

        /**
         * Decodes a Timestamp message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.Timestamp
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.Timestamp} Timestamp
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Timestamp.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.Timestamp();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.millis = reader.int64();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for Timestamp
         * @function getTypeUrl
         * @memberof petajournal.Timestamp
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Timestamp.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.Timestamp";
        };

        return Timestamp;
    })();

    petajournal.Page = (function() {

        /**
         * Properties of a Page.
         * @memberof petajournal
         * @interface IPage
         * @property {string|null} [id] Page id
         * @property {string|null} [dirPath] Page dirPath
         * @property {string|null} [fileName] Page fileName
         * @property {string|null} [type] Page type
         * @property {petajournal.ITimestamp|null} [createdDate] Page createdDate
         * @property {petajournal.ITimestamp|null} [modifiedDate] Page modifiedDate
         * @property {Array.<string>|null} [tags] Page tags
         * @property {string|null} [propertiesJson] Page propertiesJson
         */

        /**
         * Constructs a new Page.
         * @memberof petajournal
         * @classdesc Represents a Page.
         * @implements IPage
         * @constructor
         * @param {petajournal.IPage=} [properties] Properties to set
         */
        function Page(properties) {
            this.tags = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Page id.
         * @member {string} id
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.id = "";

        /**
         * Page dirPath.
         * @member {string} dirPath
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.dirPath = "";

        /**
         * Page fileName.
         * @member {string} fileName
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.fileName = "";

        /**
         * Page type.
         * @member {string} type
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.type = "";

        /**
         * Page createdDate.
         * @member {petajournal.ITimestamp|null|undefined} createdDate
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.createdDate = null;

        /**
         * Page modifiedDate.
         * @member {petajournal.ITimestamp|null|undefined} modifiedDate
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.modifiedDate = null;

        /**
         * Page tags.
         * @member {Array.<string>} tags
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.tags = $util.emptyArray;

        /**
         * Page propertiesJson.
         * @member {string} propertiesJson
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.propertiesJson = "";

        /**
         * Creates a new Page instance using the specified properties.
         * @function create
         * @memberof petajournal.Page
         * @static
         * @param {petajournal.IPage=} [properties] Properties to set
         * @returns {petajournal.Page} Page instance
         */
        Page.create = function create(properties) {
            return new Page(properties);
        };

        /**
         * Encodes the specified Page message. Does not implicitly {@link petajournal.Page.verify|verify} messages.
         * @function encode
         * @memberof petajournal.Page
         * @static
         * @param {petajournal.IPage} message Page message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Page.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.id);
            if (message.dirPath != null && Object.hasOwnProperty.call(message, "dirPath"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.dirPath);
            if (message.fileName != null && Object.hasOwnProperty.call(message, "fileName"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.fileName);
            if (message.createdDate != null && Object.hasOwnProperty.call(message, "createdDate"))
                $root.petajournal.Timestamp.encode(message.createdDate, writer.uint32(/* id 4, wireType 2 =*/34).fork()).ldelim();
            if (message.modifiedDate != null && Object.hasOwnProperty.call(message, "modifiedDate"))
                $root.petajournal.Timestamp.encode(message.modifiedDate, writer.uint32(/* id 5, wireType 2 =*/42).fork()).ldelim();
            if (message.tags != null && message.tags.length)
                for (let i = 0; i < message.tags.length; ++i)
                    writer.uint32(/* id 6, wireType 2 =*/50).string(message.tags[i]);
            if (message.propertiesJson != null && Object.hasOwnProperty.call(message, "propertiesJson"))
                writer.uint32(/* id 8, wireType 2 =*/66).string(message.propertiesJson);
            if (message.type != null && Object.hasOwnProperty.call(message, "type"))
                writer.uint32(/* id 9, wireType 2 =*/74).string(message.type);
            return writer;
        };

        /**
         * Decodes a Page message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.Page
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.Page} Page
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Page.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.Page();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.id = reader.string();
                        break;
                    }
                case 2: {
                        message.dirPath = reader.string();
                        break;
                    }
                case 3: {
                        message.fileName = reader.string();
                        break;
                    }
                case 9: {
                        message.type = reader.string();
                        break;
                    }
                case 4: {
                        message.createdDate = $root.petajournal.Timestamp.decode(reader, reader.uint32());
                        break;
                    }
                case 5: {
                        message.modifiedDate = $root.petajournal.Timestamp.decode(reader, reader.uint32());
                        break;
                    }
                case 6: {
                        if (!(message.tags && message.tags.length))
                            message.tags = [];
                        message.tags.push(reader.string());
                        break;
                    }
                case 8: {
                        message.propertiesJson = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for Page
         * @function getTypeUrl
         * @memberof petajournal.Page
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Page.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.Page";
        };

        return Page;
    })();

    petajournal.Block = (function() {

        /**
         * Properties of a Block.
         * @memberof petajournal
         * @interface IBlock
         * @property {string|null} [id] Block id
         * @property {string|null} [pageId] Block pageId
         * @property {string|null} [dirPath] Block dirPath
         * @property {string|null} [fileName] Block fileName
         * @property {string|null} [type] Block type
         * @property {petajournal.ITimestamp|null} [createdDate] Block createdDate
         * @property {petajournal.ITimestamp|null} [modifiedDate] Block modifiedDate
         * @property {string|null} [idx] Block idx
         * @property {number|null} [level] Block level
         * @property {string|null} [body] Block body
         * @property {string|null} [html] Block html
         * @property {boolean|null} [collapsed] Block collapsed
         * @property {string|null} [status] Block status
         * @property {Array.<string>|null} [types] Block types
         * @property {string|null} [propertiesJson] Block propertiesJson
         */

        /**
         * Constructs a new Block.
         * @memberof petajournal
         * @classdesc Represents a Block.
         * @implements IBlock
         * @constructor
         * @param {petajournal.IBlock=} [properties] Properties to set
         */
        function Block(properties) {
            this.types = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Block id.
         * @member {string} id
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.id = "";

        /**
         * Block pageId.
         * @member {string} pageId
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.pageId = "";

        /**
         * Block dirPath.
         * @member {string} dirPath
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.dirPath = "";

        /**
         * Block fileName.
         * @member {string} fileName
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.fileName = "";

        /**
         * Block type.
         * @member {string} type
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.type = "";

        /**
         * Block createdDate.
         * @member {petajournal.ITimestamp|null|undefined} createdDate
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.createdDate = null;

        /**
         * Block modifiedDate.
         * @member {petajournal.ITimestamp|null|undefined} modifiedDate
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.modifiedDate = null;

        /**
         * Block idx.
         * @member {string} idx
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.idx = "";

        /**
         * Block level.
         * @member {number} level
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.level = 0;

        /**
         * Block body.
         * @member {string} body
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.body = "";

        /**
         * Block html.
         * @member {string} html
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.html = "";

        /**
         * Block collapsed.
         * @member {boolean} collapsed
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.collapsed = false;

        /**
         * Block status.
         * @member {string} status
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.status = "";

        /**
         * Block types.
         * @member {Array.<string>} types
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.types = $util.emptyArray;

        /**
         * Block propertiesJson.
         * @member {string} propertiesJson
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.propertiesJson = "";

        /**
         * Creates a new Block instance using the specified properties.
         * @function create
         * @memberof petajournal.Block
         * @static
         * @param {petajournal.IBlock=} [properties] Properties to set
         * @returns {petajournal.Block} Block instance
         */
        Block.create = function create(properties) {
            return new Block(properties);
        };

        /**
         * Encodes the specified Block message. Does not implicitly {@link petajournal.Block.verify|verify} messages.
         * @function encode
         * @memberof petajournal.Block
         * @static
         * @param {petajournal.IBlock} message Block message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Block.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.id);
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.pageId);
            if (message.dirPath != null && Object.hasOwnProperty.call(message, "dirPath"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.dirPath);
            if (message.fileName != null && Object.hasOwnProperty.call(message, "fileName"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.fileName);
            if (message.createdDate != null && Object.hasOwnProperty.call(message, "createdDate"))
                $root.petajournal.Timestamp.encode(message.createdDate, writer.uint32(/* id 5, wireType 2 =*/42).fork()).ldelim();
            if (message.modifiedDate != null && Object.hasOwnProperty.call(message, "modifiedDate"))
                $root.petajournal.Timestamp.encode(message.modifiedDate, writer.uint32(/* id 6, wireType 2 =*/50).fork()).ldelim();
            if (message.idx != null && Object.hasOwnProperty.call(message, "idx"))
                writer.uint32(/* id 7, wireType 2 =*/58).string(message.idx);
            if (message.level != null && Object.hasOwnProperty.call(message, "level"))
                writer.uint32(/* id 8, wireType 0 =*/64).int32(message.level);
            if (message.body != null && Object.hasOwnProperty.call(message, "body"))
                writer.uint32(/* id 9, wireType 2 =*/74).string(message.body);
            if (message.html != null && Object.hasOwnProperty.call(message, "html"))
                writer.uint32(/* id 10, wireType 2 =*/82).string(message.html);
            if (message.collapsed != null && Object.hasOwnProperty.call(message, "collapsed"))
                writer.uint32(/* id 11, wireType 0 =*/88).bool(message.collapsed);
            if (message.status != null && Object.hasOwnProperty.call(message, "status"))
                writer.uint32(/* id 12, wireType 2 =*/98).string(message.status);
            if (message.types != null && message.types.length)
                for (let i = 0; i < message.types.length; ++i)
                    writer.uint32(/* id 13, wireType 2 =*/106).string(message.types[i]);
            if (message.propertiesJson != null && Object.hasOwnProperty.call(message, "propertiesJson"))
                writer.uint32(/* id 14, wireType 2 =*/114).string(message.propertiesJson);
            if (message.type != null && Object.hasOwnProperty.call(message, "type"))
                writer.uint32(/* id 15, wireType 2 =*/122).string(message.type);
            return writer;
        };

        /**
         * Decodes a Block message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.Block
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.Block} Block
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Block.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.Block();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.id = reader.string();
                        break;
                    }
                case 2: {
                        message.pageId = reader.string();
                        break;
                    }
                case 3: {
                        message.dirPath = reader.string();
                        break;
                    }
                case 4: {
                        message.fileName = reader.string();
                        break;
                    }
                case 15: {
                        message.type = reader.string();
                        break;
                    }
                case 5: {
                        message.createdDate = $root.petajournal.Timestamp.decode(reader, reader.uint32());
                        break;
                    }
                case 6: {
                        message.modifiedDate = $root.petajournal.Timestamp.decode(reader, reader.uint32());
                        break;
                    }
                case 7: {
                        message.idx = reader.string();
                        break;
                    }
                case 8: {
                        message.level = reader.int32();
                        break;
                    }
                case 9: {
                        message.body = reader.string();
                        break;
                    }
                case 10: {
                        message.html = reader.string();
                        break;
                    }
                case 11: {
                        message.collapsed = reader.bool();
                        break;
                    }
                case 12: {
                        message.status = reader.string();
                        break;
                    }
                case 13: {
                        if (!(message.types && message.types.length))
                            message.types = [];
                        message.types.push(reader.string());
                        break;
                    }
                case 14: {
                        message.propertiesJson = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for Block
         * @function getTypeUrl
         * @memberof petajournal.Block
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Block.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.Block";
        };

        return Block;
    })();

    petajournal.SearchRequest = (function() {

        /**
         * Properties of a SearchRequest.
         * @memberof petajournal
         * @interface ISearchRequest
         * @property {string|null} [query] SearchRequest query
         * @property {number|null} [maxResults] SearchRequest maxResults
         */

        /**
         * Constructs a new SearchRequest.
         * @memberof petajournal
         * @classdesc Represents a SearchRequest.
         * @implements ISearchRequest
         * @constructor
         * @param {petajournal.ISearchRequest=} [properties] Properties to set
         */
        function SearchRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchRequest query.
         * @member {string} query
         * @memberof petajournal.SearchRequest
         * @instance
         */
        SearchRequest.prototype.query = "";

        /**
         * SearchRequest maxResults.
         * @member {number} maxResults
         * @memberof petajournal.SearchRequest
         * @instance
         */
        SearchRequest.prototype.maxResults = 0;

        /**
         * Creates a new SearchRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchRequest
         * @static
         * @param {petajournal.ISearchRequest=} [properties] Properties to set
         * @returns {petajournal.SearchRequest} SearchRequest instance
         */
        SearchRequest.create = function create(properties) {
            return new SearchRequest(properties);
        };

        /**
         * Encodes the specified SearchRequest message. Does not implicitly {@link petajournal.SearchRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchRequest
         * @static
         * @param {petajournal.ISearchRequest} message SearchRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.query != null && Object.hasOwnProperty.call(message, "query"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.query);
            if (message.maxResults != null && Object.hasOwnProperty.call(message, "maxResults"))
                writer.uint32(/* id 2, wireType 0 =*/16).int32(message.maxResults);
            return writer;
        };

        /**
         * Decodes a SearchRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchRequest} SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.query = reader.string();
                        break;
                    }
                case 2: {
                        message.maxResults = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchRequest
         * @function getTypeUrl
         * @memberof petajournal.SearchRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchRequest";
        };

        return SearchRequest;
    })();

    petajournal.SearchResponse = (function() {

        /**
         * Properties of a SearchResponse.
         * @memberof petajournal
         * @interface ISearchResponse
         * @property {Array.<petajournal.ISearchResult>|null} [results] SearchResponse results
         * @property {number|null} [totalHits] SearchResponse totalHits
         */

        /**
         * Constructs a new SearchResponse.
         * @memberof petajournal
         * @classdesc Represents a SearchResponse.
         * @implements ISearchResponse
         * @constructor
         * @param {petajournal.ISearchResponse=} [properties] Properties to set
         */
        function SearchResponse(properties) {
            this.results = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchResponse results.
         * @member {Array.<petajournal.ISearchResult>} results
         * @memberof petajournal.SearchResponse
         * @instance
         */
        SearchResponse.prototype.results = $util.emptyArray;

        /**
         * SearchResponse totalHits.
         * @member {number} totalHits
         * @memberof petajournal.SearchResponse
         * @instance
         */
        SearchResponse.prototype.totalHits = 0;

        /**
         * Creates a new SearchResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchResponse
         * @static
         * @param {petajournal.ISearchResponse=} [properties] Properties to set
         * @returns {petajournal.SearchResponse} SearchResponse instance
         */
        SearchResponse.create = function create(properties) {
            return new SearchResponse(properties);
        };

        /**
         * Encodes the specified SearchResponse message. Does not implicitly {@link petajournal.SearchResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchResponse
         * @static
         * @param {petajournal.ISearchResponse} message SearchResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.results != null && message.results.length)
                for (let i = 0; i < message.results.length; ++i)
                    $root.petajournal.SearchResult.encode(message.results[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.totalHits != null && Object.hasOwnProperty.call(message, "totalHits"))
                writer.uint32(/* id 2, wireType 0 =*/16).int32(message.totalHits);
            return writer;
        };

        /**
         * Decodes a SearchResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchResponse} SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.results && message.results.length))
                            message.results = [];
                        message.results.push($root.petajournal.SearchResult.decode(reader, reader.uint32()));
                        break;
                    }
                case 2: {
                        message.totalHits = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchResponse
         * @function getTypeUrl
         * @memberof petajournal.SearchResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchResponse";
        };

        return SearchResponse;
    })();

    petajournal.SearchResult = (function() {

        /**
         * Properties of a SearchResult.
         * @memberof petajournal
         * @interface ISearchResult
         * @property {string|null} [pageId] SearchResult pageId
         * @property {string|null} [pageTitle] SearchResult pageTitle
         * @property {string|null} [blockId] SearchResult blockId
         * @property {string|null} [snippet] SearchResult snippet
         * @property {number|null} [score] SearchResult score
         */

        /**
         * Constructs a new SearchResult.
         * @memberof petajournal
         * @classdesc Represents a SearchResult.
         * @implements ISearchResult
         * @constructor
         * @param {petajournal.ISearchResult=} [properties] Properties to set
         */
        function SearchResult(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchResult pageId.
         * @member {string} pageId
         * @memberof petajournal.SearchResult
         * @instance
         */
        SearchResult.prototype.pageId = "";

        /**
         * SearchResult pageTitle.
         * @member {string} pageTitle
         * @memberof petajournal.SearchResult
         * @instance
         */
        SearchResult.prototype.pageTitle = "";

        /**
         * SearchResult blockId.
         * @member {string} blockId
         * @memberof petajournal.SearchResult
         * @instance
         */
        SearchResult.prototype.blockId = "";

        /**
         * SearchResult snippet.
         * @member {string} snippet
         * @memberof petajournal.SearchResult
         * @instance
         */
        SearchResult.prototype.snippet = "";

        /**
         * SearchResult score.
         * @member {number} score
         * @memberof petajournal.SearchResult
         * @instance
         */
        SearchResult.prototype.score = 0;

        /**
         * Creates a new SearchResult instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchResult
         * @static
         * @param {petajournal.ISearchResult=} [properties] Properties to set
         * @returns {petajournal.SearchResult} SearchResult instance
         */
        SearchResult.create = function create(properties) {
            return new SearchResult(properties);
        };

        /**
         * Encodes the specified SearchResult message. Does not implicitly {@link petajournal.SearchResult.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchResult
         * @static
         * @param {petajournal.ISearchResult} message SearchResult message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchResult.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            if (message.pageTitle != null && Object.hasOwnProperty.call(message, "pageTitle"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.pageTitle);
            if (message.blockId != null && Object.hasOwnProperty.call(message, "blockId"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.blockId);
            if (message.snippet != null && Object.hasOwnProperty.call(message, "snippet"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.snippet);
            if (message.score != null && Object.hasOwnProperty.call(message, "score"))
                writer.uint32(/* id 5, wireType 5 =*/45).float(message.score);
            return writer;
        };

        /**
         * Decodes a SearchResult message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchResult
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchResult} SearchResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchResult.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchResult();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                case 2: {
                        message.pageTitle = reader.string();
                        break;
                    }
                case 3: {
                        message.blockId = reader.string();
                        break;
                    }
                case 4: {
                        message.snippet = reader.string();
                        break;
                    }
                case 5: {
                        message.score = reader.float();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchResult
         * @function getTypeUrl
         * @memberof petajournal.SearchResult
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchResult.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchResult";
        };

        return SearchResult;
    })();

    petajournal.ApiRequest = (function() {

        /**
         * Properties of an ApiRequest.
         * @memberof petajournal
         * @interface IApiRequest
         * @property {string|null} [requestId] ApiRequest requestId
         * @property {petajournal.IGetPageRequest|null} [getPage] ApiRequest getPage
         * @property {petajournal.IGetDocByNameRequest|null} [getDocByName] ApiRequest getDocByName
         * @property {petajournal.IDeletePageRequest|null} [deletePage] ApiRequest deletePage
         * @property {petajournal.IUpdatePageRequest|null} [updatePage] ApiRequest updatePage
         * @property {petajournal.ICreateBlockRequest|null} [createBlock] ApiRequest createBlock
         * @property {petajournal.IUpdateBlockRequest|null} [updateBlock] ApiRequest updateBlock
         * @property {petajournal.IMoveBlocksRequest|null} [moveBlocks] ApiRequest moveBlocks
         * @property {petajournal.IDeleteBlockRequest|null} [deleteBlock] ApiRequest deleteBlock
         * @property {petajournal.IUpdateBlockStatusRequest|null} [updateBlockStatus] ApiRequest updateBlockStatus
         * @property {petajournal.ISearchPageNamesRequest|null} [searchPageNames] ApiRequest searchPageNames
         * @property {petajournal.ISearchFullTextRequest|null} [searchFullText] ApiRequest searchFullText
         * @property {petajournal.IGetRecentPagesRequest|null} [getRecentPages] ApiRequest getRecentPages
         * @property {petajournal.IGetBacklinksRequest|null} [getBacklinks] ApiRequest getBacklinks
         * @property {petajournal.IGetMostRecentJournalBeforeRequest|null} [getMostRecentJournalBefore] ApiRequest getMostRecentJournalBefore
         * @property {petajournal.IGetBlocksByStatusRequest|null} [getBlocksByStatus] ApiRequest getBlocksByStatus
         * @property {petajournal.IGetConfigRequest|null} [getConfig] ApiRequest getConfig
         * @property {petajournal.IUpdateConfigRequest|null} [updateConfig] ApiRequest updateConfig
         * @property {petajournal.IGetJournalByNameRequest|null} [getJournalByName] ApiRequest getJournalByName
         * @property {petajournal.IExportTreeMarkdownRequest|null} [exportTreeMarkdown] ApiRequest exportTreeMarkdown
         * @property {petajournal.IImportTreeMarkdownRequest|null} [importTreeMarkdown] ApiRequest importTreeMarkdown
         */

        /**
         * Constructs a new ApiRequest.
         * @memberof petajournal
         * @classdesc Represents an ApiRequest.
         * @implements IApiRequest
         * @constructor
         * @param {petajournal.IApiRequest=} [properties] Properties to set
         */
        function ApiRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ApiRequest requestId.
         * @member {string} requestId
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.requestId = "";

        /**
         * ApiRequest getPage.
         * @member {petajournal.IGetPageRequest|null|undefined} getPage
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getPage = null;

        /**
         * ApiRequest getDocByName.
         * @member {petajournal.IGetDocByNameRequest|null|undefined} getDocByName
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getDocByName = null;

        /**
         * ApiRequest deletePage.
         * @member {petajournal.IDeletePageRequest|null|undefined} deletePage
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.deletePage = null;

        /**
         * ApiRequest updatePage.
         * @member {petajournal.IUpdatePageRequest|null|undefined} updatePage
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.updatePage = null;

        /**
         * ApiRequest createBlock.
         * @member {petajournal.ICreateBlockRequest|null|undefined} createBlock
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.createBlock = null;

        /**
         * ApiRequest updateBlock.
         * @member {petajournal.IUpdateBlockRequest|null|undefined} updateBlock
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.updateBlock = null;

        /**
         * ApiRequest moveBlocks.
         * @member {petajournal.IMoveBlocksRequest|null|undefined} moveBlocks
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.moveBlocks = null;

        /**
         * ApiRequest deleteBlock.
         * @member {petajournal.IDeleteBlockRequest|null|undefined} deleteBlock
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.deleteBlock = null;

        /**
         * ApiRequest updateBlockStatus.
         * @member {petajournal.IUpdateBlockStatusRequest|null|undefined} updateBlockStatus
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.updateBlockStatus = null;

        /**
         * ApiRequest searchPageNames.
         * @member {petajournal.ISearchPageNamesRequest|null|undefined} searchPageNames
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.searchPageNames = null;

        /**
         * ApiRequest searchFullText.
         * @member {petajournal.ISearchFullTextRequest|null|undefined} searchFullText
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.searchFullText = null;

        /**
         * ApiRequest getRecentPages.
         * @member {petajournal.IGetRecentPagesRequest|null|undefined} getRecentPages
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getRecentPages = null;

        /**
         * ApiRequest getBacklinks.
         * @member {petajournal.IGetBacklinksRequest|null|undefined} getBacklinks
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getBacklinks = null;

        /**
         * ApiRequest getMostRecentJournalBefore.
         * @member {petajournal.IGetMostRecentJournalBeforeRequest|null|undefined} getMostRecentJournalBefore
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getMostRecentJournalBefore = null;

        /**
         * ApiRequest getBlocksByStatus.
         * @member {petajournal.IGetBlocksByStatusRequest|null|undefined} getBlocksByStatus
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getBlocksByStatus = null;

        /**
         * ApiRequest getConfig.
         * @member {petajournal.IGetConfigRequest|null|undefined} getConfig
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getConfig = null;

        /**
         * ApiRequest updateConfig.
         * @member {petajournal.IUpdateConfigRequest|null|undefined} updateConfig
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.updateConfig = null;

        /**
         * ApiRequest getJournalByName.
         * @member {petajournal.IGetJournalByNameRequest|null|undefined} getJournalByName
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getJournalByName = null;

        /**
         * ApiRequest exportTreeMarkdown.
         * @member {petajournal.IExportTreeMarkdownRequest|null|undefined} exportTreeMarkdown
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.exportTreeMarkdown = null;

        /**
         * ApiRequest importTreeMarkdown.
         * @member {petajournal.IImportTreeMarkdownRequest|null|undefined} importTreeMarkdown
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.importTreeMarkdown = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        /**
         * ApiRequest payload.
         * @member {"getPage"|"getDocByName"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getBlocksByStatus"|"getConfig"|"updateConfig"|"getJournalByName"|"exportTreeMarkdown"|"importTreeMarkdown"|undefined} payload
         * @memberof petajournal.ApiRequest
         * @instance
         */
        Object.defineProperty(ApiRequest.prototype, "payload", {
            get: $util.oneOfGetter($oneOfFields = ["getPage", "getDocByName", "deletePage", "updatePage", "createBlock", "updateBlock", "moveBlocks", "deleteBlock", "updateBlockStatus", "searchPageNames", "searchFullText", "getRecentPages", "getBacklinks", "getMostRecentJournalBefore", "getBlocksByStatus", "getConfig", "updateConfig", "getJournalByName", "exportTreeMarkdown", "importTreeMarkdown"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new ApiRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.ApiRequest
         * @static
         * @param {petajournal.IApiRequest=} [properties] Properties to set
         * @returns {petajournal.ApiRequest} ApiRequest instance
         */
        ApiRequest.create = function create(properties) {
            return new ApiRequest(properties);
        };

        /**
         * Encodes the specified ApiRequest message. Does not implicitly {@link petajournal.ApiRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ApiRequest
         * @static
         * @param {petajournal.IApiRequest} message ApiRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ApiRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.requestId != null && Object.hasOwnProperty.call(message, "requestId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.requestId);
            if (message.getPage != null && Object.hasOwnProperty.call(message, "getPage"))
                $root.petajournal.GetPageRequest.encode(message.getPage, writer.uint32(/* id 10, wireType 2 =*/82).fork()).ldelim();
            if (message.getDocByName != null && Object.hasOwnProperty.call(message, "getDocByName"))
                $root.petajournal.GetDocByNameRequest.encode(message.getDocByName, writer.uint32(/* id 12, wireType 2 =*/98).fork()).ldelim();
            if (message.deletePage != null && Object.hasOwnProperty.call(message, "deletePage"))
                $root.petajournal.DeletePageRequest.encode(message.deletePage, writer.uint32(/* id 13, wireType 2 =*/106).fork()).ldelim();
            if (message.updatePage != null && Object.hasOwnProperty.call(message, "updatePage"))
                $root.petajournal.UpdatePageRequest.encode(message.updatePage, writer.uint32(/* id 14, wireType 2 =*/114).fork()).ldelim();
            if (message.createBlock != null && Object.hasOwnProperty.call(message, "createBlock"))
                $root.petajournal.CreateBlockRequest.encode(message.createBlock, writer.uint32(/* id 20, wireType 2 =*/162).fork()).ldelim();
            if (message.updateBlock != null && Object.hasOwnProperty.call(message, "updateBlock"))
                $root.petajournal.UpdateBlockRequest.encode(message.updateBlock, writer.uint32(/* id 21, wireType 2 =*/170).fork()).ldelim();
            if (message.moveBlocks != null && Object.hasOwnProperty.call(message, "moveBlocks"))
                $root.petajournal.MoveBlocksRequest.encode(message.moveBlocks, writer.uint32(/* id 22, wireType 2 =*/178).fork()).ldelim();
            if (message.deleteBlock != null && Object.hasOwnProperty.call(message, "deleteBlock"))
                $root.petajournal.DeleteBlockRequest.encode(message.deleteBlock, writer.uint32(/* id 23, wireType 2 =*/186).fork()).ldelim();
            if (message.updateBlockStatus != null && Object.hasOwnProperty.call(message, "updateBlockStatus"))
                $root.petajournal.UpdateBlockStatusRequest.encode(message.updateBlockStatus, writer.uint32(/* id 24, wireType 2 =*/194).fork()).ldelim();
            if (message.searchPageNames != null && Object.hasOwnProperty.call(message, "searchPageNames"))
                $root.petajournal.SearchPageNamesRequest.encode(message.searchPageNames, writer.uint32(/* id 30, wireType 2 =*/242).fork()).ldelim();
            if (message.searchFullText != null && Object.hasOwnProperty.call(message, "searchFullText"))
                $root.petajournal.SearchFullTextRequest.encode(message.searchFullText, writer.uint32(/* id 31, wireType 2 =*/250).fork()).ldelim();
            if (message.getRecentPages != null && Object.hasOwnProperty.call(message, "getRecentPages"))
                $root.petajournal.GetRecentPagesRequest.encode(message.getRecentPages, writer.uint32(/* id 40, wireType 2 =*/322).fork()).ldelim();
            if (message.getBacklinks != null && Object.hasOwnProperty.call(message, "getBacklinks"))
                $root.petajournal.GetBacklinksRequest.encode(message.getBacklinks, writer.uint32(/* id 41, wireType 2 =*/330).fork()).ldelim();
            if (message.getMostRecentJournalBefore != null && Object.hasOwnProperty.call(message, "getMostRecentJournalBefore"))
                $root.petajournal.GetMostRecentJournalBeforeRequest.encode(message.getMostRecentJournalBefore, writer.uint32(/* id 42, wireType 2 =*/338).fork()).ldelim();
            if (message.getJournalByName != null && Object.hasOwnProperty.call(message, "getJournalByName"))
                $root.petajournal.GetJournalByNameRequest.encode(message.getJournalByName, writer.uint32(/* id 44, wireType 2 =*/354).fork()).ldelim();
            if (message.getBlocksByStatus != null && Object.hasOwnProperty.call(message, "getBlocksByStatus"))
                $root.petajournal.GetBlocksByStatusRequest.encode(message.getBlocksByStatus, writer.uint32(/* id 50, wireType 2 =*/402).fork()).ldelim();
            if (message.getConfig != null && Object.hasOwnProperty.call(message, "getConfig"))
                $root.petajournal.GetConfigRequest.encode(message.getConfig, writer.uint32(/* id 60, wireType 2 =*/482).fork()).ldelim();
            if (message.updateConfig != null && Object.hasOwnProperty.call(message, "updateConfig"))
                $root.petajournal.UpdateConfigRequest.encode(message.updateConfig, writer.uint32(/* id 61, wireType 2 =*/490).fork()).ldelim();
            if (message.exportTreeMarkdown != null && Object.hasOwnProperty.call(message, "exportTreeMarkdown"))
                $root.petajournal.ExportTreeMarkdownRequest.encode(message.exportTreeMarkdown, writer.uint32(/* id 70, wireType 2 =*/562).fork()).ldelim();
            if (message.importTreeMarkdown != null && Object.hasOwnProperty.call(message, "importTreeMarkdown"))
                $root.petajournal.ImportTreeMarkdownRequest.encode(message.importTreeMarkdown, writer.uint32(/* id 71, wireType 2 =*/570).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes an ApiRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ApiRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ApiRequest} ApiRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ApiRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ApiRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.requestId = reader.string();
                        break;
                    }
                case 10: {
                        message.getPage = $root.petajournal.GetPageRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 12: {
                        message.getDocByName = $root.petajournal.GetDocByNameRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 13: {
                        message.deletePage = $root.petajournal.DeletePageRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 14: {
                        message.updatePage = $root.petajournal.UpdatePageRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 20: {
                        message.createBlock = $root.petajournal.CreateBlockRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 21: {
                        message.updateBlock = $root.petajournal.UpdateBlockRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 22: {
                        message.moveBlocks = $root.petajournal.MoveBlocksRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 23: {
                        message.deleteBlock = $root.petajournal.DeleteBlockRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 24: {
                        message.updateBlockStatus = $root.petajournal.UpdateBlockStatusRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 30: {
                        message.searchPageNames = $root.petajournal.SearchPageNamesRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 31: {
                        message.searchFullText = $root.petajournal.SearchFullTextRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 40: {
                        message.getRecentPages = $root.petajournal.GetRecentPagesRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 41: {
                        message.getBacklinks = $root.petajournal.GetBacklinksRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 42: {
                        message.getMostRecentJournalBefore = $root.petajournal.GetMostRecentJournalBeforeRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 50: {
                        message.getBlocksByStatus = $root.petajournal.GetBlocksByStatusRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 60: {
                        message.getConfig = $root.petajournal.GetConfigRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 61: {
                        message.updateConfig = $root.petajournal.UpdateConfigRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 44: {
                        message.getJournalByName = $root.petajournal.GetJournalByNameRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 70: {
                        message.exportTreeMarkdown = $root.petajournal.ExportTreeMarkdownRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 71: {
                        message.importTreeMarkdown = $root.petajournal.ImportTreeMarkdownRequest.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ApiRequest
         * @function getTypeUrl
         * @memberof petajournal.ApiRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ApiRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ApiRequest";
        };

        return ApiRequest;
    })();

    petajournal.ApiResponse = (function() {

        /**
         * Properties of an ApiResponse.
         * @memberof petajournal
         * @interface IApiResponse
         * @property {string|null} [requestId] ApiResponse requestId
         * @property {boolean|null} [success] ApiResponse success
         * @property {string|null} [errorMessage] ApiResponse errorMessage
         * @property {petajournal.IGetPageResponse|null} [getPage] ApiResponse getPage
         * @property {petajournal.IGetDocByNameResponse|null} [getDocByName] ApiResponse getDocByName
         * @property {petajournal.IDeletePageResponse|null} [deletePage] ApiResponse deletePage
         * @property {petajournal.IUpdatePageResponse|null} [updatePage] ApiResponse updatePage
         * @property {petajournal.ICreateBlockResponse|null} [createBlock] ApiResponse createBlock
         * @property {petajournal.IUpdateBlockResponse|null} [updateBlock] ApiResponse updateBlock
         * @property {petajournal.IMoveBlocksResponse|null} [moveBlocks] ApiResponse moveBlocks
         * @property {petajournal.IDeleteBlockResponse|null} [deleteBlock] ApiResponse deleteBlock
         * @property {petajournal.IUpdateBlockStatusResponse|null} [updateBlockStatus] ApiResponse updateBlockStatus
         * @property {petajournal.ISearchPageNamesResponse|null} [searchPageNames] ApiResponse searchPageNames
         * @property {petajournal.ISearchFullTextResponse|null} [searchFullText] ApiResponse searchFullText
         * @property {petajournal.IGetRecentPagesResponse|null} [getRecentPages] ApiResponse getRecentPages
         * @property {petajournal.IGetBacklinksResponse|null} [getBacklinks] ApiResponse getBacklinks
         * @property {petajournal.IGetMostRecentJournalBeforeResponse|null} [getMostRecentJournalBefore] ApiResponse getMostRecentJournalBefore
         * @property {petajournal.IGetJournalByNameResponse|null} [getJournalByName] ApiResponse getJournalByName
         * @property {petajournal.IGetBlocksByStatusResponse|null} [getBlocksByStatus] ApiResponse getBlocksByStatus
         * @property {petajournal.IGetConfigResponse|null} [getConfig] ApiResponse getConfig
         * @property {petajournal.IUpdateConfigResponse|null} [updateConfig] ApiResponse updateConfig
         * @property {petajournal.IExportTreeMarkdownResponse|null} [exportTreeMarkdown] ApiResponse exportTreeMarkdown
         * @property {petajournal.IImportTreeMarkdownResponse|null} [importTreeMarkdown] ApiResponse importTreeMarkdown
         * @property {petajournal.IServerEvent|null} [serverEvent] ApiResponse serverEvent
         */

        /**
         * Constructs a new ApiResponse.
         * @memberof petajournal
         * @classdesc Represents an ApiResponse.
         * @implements IApiResponse
         * @constructor
         * @param {petajournal.IApiResponse=} [properties] Properties to set
         */
        function ApiResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ApiResponse requestId.
         * @member {string} requestId
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.requestId = "";

        /**
         * ApiResponse success.
         * @member {boolean} success
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.success = false;

        /**
         * ApiResponse errorMessage.
         * @member {string} errorMessage
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.errorMessage = "";

        /**
         * ApiResponse getPage.
         * @member {petajournal.IGetPageResponse|null|undefined} getPage
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getPage = null;

        /**
         * ApiResponse getDocByName.
         * @member {petajournal.IGetDocByNameResponse|null|undefined} getDocByName
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getDocByName = null;

        /**
         * ApiResponse deletePage.
         * @member {petajournal.IDeletePageResponse|null|undefined} deletePage
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.deletePage = null;

        /**
         * ApiResponse updatePage.
         * @member {petajournal.IUpdatePageResponse|null|undefined} updatePage
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.updatePage = null;

        /**
         * ApiResponse createBlock.
         * @member {petajournal.ICreateBlockResponse|null|undefined} createBlock
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.createBlock = null;

        /**
         * ApiResponse updateBlock.
         * @member {petajournal.IUpdateBlockResponse|null|undefined} updateBlock
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.updateBlock = null;

        /**
         * ApiResponse moveBlocks.
         * @member {petajournal.IMoveBlocksResponse|null|undefined} moveBlocks
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.moveBlocks = null;

        /**
         * ApiResponse deleteBlock.
         * @member {petajournal.IDeleteBlockResponse|null|undefined} deleteBlock
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.deleteBlock = null;

        /**
         * ApiResponse updateBlockStatus.
         * @member {petajournal.IUpdateBlockStatusResponse|null|undefined} updateBlockStatus
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.updateBlockStatus = null;

        /**
         * ApiResponse searchPageNames.
         * @member {petajournal.ISearchPageNamesResponse|null|undefined} searchPageNames
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.searchPageNames = null;

        /**
         * ApiResponse searchFullText.
         * @member {petajournal.ISearchFullTextResponse|null|undefined} searchFullText
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.searchFullText = null;

        /**
         * ApiResponse getRecentPages.
         * @member {petajournal.IGetRecentPagesResponse|null|undefined} getRecentPages
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getRecentPages = null;

        /**
         * ApiResponse getBacklinks.
         * @member {petajournal.IGetBacklinksResponse|null|undefined} getBacklinks
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getBacklinks = null;

        /**
         * ApiResponse getMostRecentJournalBefore.
         * @member {petajournal.IGetMostRecentJournalBeforeResponse|null|undefined} getMostRecentJournalBefore
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getMostRecentJournalBefore = null;

        /**
         * ApiResponse getJournalByName.
         * @member {petajournal.IGetJournalByNameResponse|null|undefined} getJournalByName
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getJournalByName = null;

        /**
         * ApiResponse getBlocksByStatus.
         * @member {petajournal.IGetBlocksByStatusResponse|null|undefined} getBlocksByStatus
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getBlocksByStatus = null;

        /**
         * ApiResponse getConfig.
         * @member {petajournal.IGetConfigResponse|null|undefined} getConfig
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getConfig = null;

        /**
         * ApiResponse updateConfig.
         * @member {petajournal.IUpdateConfigResponse|null|undefined} updateConfig
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.updateConfig = null;

        /**
         * ApiResponse exportTreeMarkdown.
         * @member {petajournal.IExportTreeMarkdownResponse|null|undefined} exportTreeMarkdown
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.exportTreeMarkdown = null;

        /**
         * ApiResponse importTreeMarkdown.
         * @member {petajournal.IImportTreeMarkdownResponse|null|undefined} importTreeMarkdown
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.importTreeMarkdown = null;

        /**
         * ApiResponse serverEvent.
         * @member {petajournal.IServerEvent|null|undefined} serverEvent
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.serverEvent = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        /**
         * ApiResponse payload.
         * @member {"getPage"|"getDocByName"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getJournalByName"|"getBlocksByStatus"|"getConfig"|"updateConfig"|"exportTreeMarkdown"|"importTreeMarkdown"|"serverEvent"|undefined} payload
         * @memberof petajournal.ApiResponse
         * @instance
         */
        Object.defineProperty(ApiResponse.prototype, "payload", {
            get: $util.oneOfGetter($oneOfFields = ["getPage", "getDocByName", "deletePage", "updatePage", "createBlock", "updateBlock", "moveBlocks", "deleteBlock", "updateBlockStatus", "searchPageNames", "searchFullText", "getRecentPages", "getBacklinks", "getMostRecentJournalBefore", "getJournalByName", "getBlocksByStatus", "getConfig", "updateConfig", "exportTreeMarkdown", "importTreeMarkdown", "serverEvent"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new ApiResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.ApiResponse
         * @static
         * @param {petajournal.IApiResponse=} [properties] Properties to set
         * @returns {petajournal.ApiResponse} ApiResponse instance
         */
        ApiResponse.create = function create(properties) {
            return new ApiResponse(properties);
        };

        /**
         * Encodes the specified ApiResponse message. Does not implicitly {@link petajournal.ApiResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ApiResponse
         * @static
         * @param {petajournal.IApiResponse} message ApiResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ApiResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.requestId != null && Object.hasOwnProperty.call(message, "requestId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.requestId);
            if (message.success != null && Object.hasOwnProperty.call(message, "success"))
                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.success);
            if (message.errorMessage != null && Object.hasOwnProperty.call(message, "errorMessage"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.errorMessage);
            if (message.getPage != null && Object.hasOwnProperty.call(message, "getPage"))
                $root.petajournal.GetPageResponse.encode(message.getPage, writer.uint32(/* id 10, wireType 2 =*/82).fork()).ldelim();
            if (message.getDocByName != null && Object.hasOwnProperty.call(message, "getDocByName"))
                $root.petajournal.GetDocByNameResponse.encode(message.getDocByName, writer.uint32(/* id 12, wireType 2 =*/98).fork()).ldelim();
            if (message.deletePage != null && Object.hasOwnProperty.call(message, "deletePage"))
                $root.petajournal.DeletePageResponse.encode(message.deletePage, writer.uint32(/* id 13, wireType 2 =*/106).fork()).ldelim();
            if (message.updatePage != null && Object.hasOwnProperty.call(message, "updatePage"))
                $root.petajournal.UpdatePageResponse.encode(message.updatePage, writer.uint32(/* id 14, wireType 2 =*/114).fork()).ldelim();
            if (message.createBlock != null && Object.hasOwnProperty.call(message, "createBlock"))
                $root.petajournal.CreateBlockResponse.encode(message.createBlock, writer.uint32(/* id 20, wireType 2 =*/162).fork()).ldelim();
            if (message.updateBlock != null && Object.hasOwnProperty.call(message, "updateBlock"))
                $root.petajournal.UpdateBlockResponse.encode(message.updateBlock, writer.uint32(/* id 21, wireType 2 =*/170).fork()).ldelim();
            if (message.moveBlocks != null && Object.hasOwnProperty.call(message, "moveBlocks"))
                $root.petajournal.MoveBlocksResponse.encode(message.moveBlocks, writer.uint32(/* id 22, wireType 2 =*/178).fork()).ldelim();
            if (message.deleteBlock != null && Object.hasOwnProperty.call(message, "deleteBlock"))
                $root.petajournal.DeleteBlockResponse.encode(message.deleteBlock, writer.uint32(/* id 23, wireType 2 =*/186).fork()).ldelim();
            if (message.updateBlockStatus != null && Object.hasOwnProperty.call(message, "updateBlockStatus"))
                $root.petajournal.UpdateBlockStatusResponse.encode(message.updateBlockStatus, writer.uint32(/* id 24, wireType 2 =*/194).fork()).ldelim();
            if (message.searchPageNames != null && Object.hasOwnProperty.call(message, "searchPageNames"))
                $root.petajournal.SearchPageNamesResponse.encode(message.searchPageNames, writer.uint32(/* id 30, wireType 2 =*/242).fork()).ldelim();
            if (message.searchFullText != null && Object.hasOwnProperty.call(message, "searchFullText"))
                $root.petajournal.SearchFullTextResponse.encode(message.searchFullText, writer.uint32(/* id 31, wireType 2 =*/250).fork()).ldelim();
            if (message.getRecentPages != null && Object.hasOwnProperty.call(message, "getRecentPages"))
                $root.petajournal.GetRecentPagesResponse.encode(message.getRecentPages, writer.uint32(/* id 40, wireType 2 =*/322).fork()).ldelim();
            if (message.getBacklinks != null && Object.hasOwnProperty.call(message, "getBacklinks"))
                $root.petajournal.GetBacklinksResponse.encode(message.getBacklinks, writer.uint32(/* id 41, wireType 2 =*/330).fork()).ldelim();
            if (message.getMostRecentJournalBefore != null && Object.hasOwnProperty.call(message, "getMostRecentJournalBefore"))
                $root.petajournal.GetMostRecentJournalBeforeResponse.encode(message.getMostRecentJournalBefore, writer.uint32(/* id 42, wireType 2 =*/338).fork()).ldelim();
            if (message.getJournalByName != null && Object.hasOwnProperty.call(message, "getJournalByName"))
                $root.petajournal.GetJournalByNameResponse.encode(message.getJournalByName, writer.uint32(/* id 44, wireType 2 =*/354).fork()).ldelim();
            if (message.getBlocksByStatus != null && Object.hasOwnProperty.call(message, "getBlocksByStatus"))
                $root.petajournal.GetBlocksByStatusResponse.encode(message.getBlocksByStatus, writer.uint32(/* id 50, wireType 2 =*/402).fork()).ldelim();
            if (message.getConfig != null && Object.hasOwnProperty.call(message, "getConfig"))
                $root.petajournal.GetConfigResponse.encode(message.getConfig, writer.uint32(/* id 60, wireType 2 =*/482).fork()).ldelim();
            if (message.updateConfig != null && Object.hasOwnProperty.call(message, "updateConfig"))
                $root.petajournal.UpdateConfigResponse.encode(message.updateConfig, writer.uint32(/* id 61, wireType 2 =*/490).fork()).ldelim();
            if (message.exportTreeMarkdown != null && Object.hasOwnProperty.call(message, "exportTreeMarkdown"))
                $root.petajournal.ExportTreeMarkdownResponse.encode(message.exportTreeMarkdown, writer.uint32(/* id 70, wireType 2 =*/562).fork()).ldelim();
            if (message.importTreeMarkdown != null && Object.hasOwnProperty.call(message, "importTreeMarkdown"))
                $root.petajournal.ImportTreeMarkdownResponse.encode(message.importTreeMarkdown, writer.uint32(/* id 71, wireType 2 =*/570).fork()).ldelim();
            if (message.serverEvent != null && Object.hasOwnProperty.call(message, "serverEvent"))
                $root.petajournal.ServerEvent.encode(message.serverEvent, writer.uint32(/* id 100, wireType 2 =*/802).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes an ApiResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ApiResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ApiResponse} ApiResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ApiResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ApiResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.requestId = reader.string();
                        break;
                    }
                case 2: {
                        message.success = reader.bool();
                        break;
                    }
                case 3: {
                        message.errorMessage = reader.string();
                        break;
                    }
                case 10: {
                        message.getPage = $root.petajournal.GetPageResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 12: {
                        message.getDocByName = $root.petajournal.GetDocByNameResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 13: {
                        message.deletePage = $root.petajournal.DeletePageResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 14: {
                        message.updatePage = $root.petajournal.UpdatePageResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 20: {
                        message.createBlock = $root.petajournal.CreateBlockResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 21: {
                        message.updateBlock = $root.petajournal.UpdateBlockResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 22: {
                        message.moveBlocks = $root.petajournal.MoveBlocksResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 23: {
                        message.deleteBlock = $root.petajournal.DeleteBlockResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 24: {
                        message.updateBlockStatus = $root.petajournal.UpdateBlockStatusResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 30: {
                        message.searchPageNames = $root.petajournal.SearchPageNamesResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 31: {
                        message.searchFullText = $root.petajournal.SearchFullTextResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 40: {
                        message.getRecentPages = $root.petajournal.GetRecentPagesResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 41: {
                        message.getBacklinks = $root.petajournal.GetBacklinksResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 42: {
                        message.getMostRecentJournalBefore = $root.petajournal.GetMostRecentJournalBeforeResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 44: {
                        message.getJournalByName = $root.petajournal.GetJournalByNameResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 50: {
                        message.getBlocksByStatus = $root.petajournal.GetBlocksByStatusResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 60: {
                        message.getConfig = $root.petajournal.GetConfigResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 61: {
                        message.updateConfig = $root.petajournal.UpdateConfigResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 70: {
                        message.exportTreeMarkdown = $root.petajournal.ExportTreeMarkdownResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 71: {
                        message.importTreeMarkdown = $root.petajournal.ImportTreeMarkdownResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 100: {
                        message.serverEvent = $root.petajournal.ServerEvent.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ApiResponse
         * @function getTypeUrl
         * @memberof petajournal.ApiResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ApiResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ApiResponse";
        };

        return ApiResponse;
    })();

    petajournal.GetPageRequest = (function() {

        /**
         * Properties of a GetPageRequest.
         * @memberof petajournal
         * @interface IGetPageRequest
         * @property {string|null} [pageId] GetPageRequest pageId
         */

        /**
         * Constructs a new GetPageRequest.
         * @memberof petajournal
         * @classdesc Represents a GetPageRequest.
         * @implements IGetPageRequest
         * @constructor
         * @param {petajournal.IGetPageRequest=} [properties] Properties to set
         */
        function GetPageRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetPageRequest pageId.
         * @member {string} pageId
         * @memberof petajournal.GetPageRequest
         * @instance
         */
        GetPageRequest.prototype.pageId = "";

        /**
         * Creates a new GetPageRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetPageRequest
         * @static
         * @param {petajournal.IGetPageRequest=} [properties] Properties to set
         * @returns {petajournal.GetPageRequest} GetPageRequest instance
         */
        GetPageRequest.create = function create(properties) {
            return new GetPageRequest(properties);
        };

        /**
         * Encodes the specified GetPageRequest message. Does not implicitly {@link petajournal.GetPageRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetPageRequest
         * @static
         * @param {petajournal.IGetPageRequest} message GetPageRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetPageRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            return writer;
        };

        /**
         * Decodes a GetPageRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetPageRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetPageRequest} GetPageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetPageRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetPageRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetPageRequest
         * @function getTypeUrl
         * @memberof petajournal.GetPageRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetPageRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetPageRequest";
        };

        return GetPageRequest;
    })();

    petajournal.GetPageResponse = (function() {

        /**
         * Properties of a GetPageResponse.
         * @memberof petajournal
         * @interface IGetPageResponse
         * @property {petajournal.IPage|null} [page] GetPageResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetPageResponse blocks
         */

        /**
         * Constructs a new GetPageResponse.
         * @memberof petajournal
         * @classdesc Represents a GetPageResponse.
         * @implements IGetPageResponse
         * @constructor
         * @param {petajournal.IGetPageResponse=} [properties] Properties to set
         */
        function GetPageResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetPageResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetPageResponse
         * @instance
         */
        GetPageResponse.prototype.page = null;

        /**
         * GetPageResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetPageResponse
         * @instance
         */
        GetPageResponse.prototype.blocks = $util.emptyArray;

        /**
         * Creates a new GetPageResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetPageResponse
         * @static
         * @param {petajournal.IGetPageResponse=} [properties] Properties to set
         * @returns {petajournal.GetPageResponse} GetPageResponse instance
         */
        GetPageResponse.create = function create(properties) {
            return new GetPageResponse(properties);
        };

        /**
         * Encodes the specified GetPageResponse message. Does not implicitly {@link petajournal.GetPageResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetPageResponse
         * @static
         * @param {petajournal.IGetPageResponse} message GetPageResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetPageResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.page != null && Object.hasOwnProperty.call(message, "page"))
                $root.petajournal.Page.encode(message.page, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.blocks != null && message.blocks.length)
                for (let i = 0; i < message.blocks.length; ++i)
                    $root.petajournal.Block.encode(message.blocks[i], writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a GetPageResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetPageResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetPageResponse} GetPageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetPageResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetPageResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.page = $root.petajournal.Page.decode(reader, reader.uint32());
                        break;
                    }
                case 2: {
                        if (!(message.blocks && message.blocks.length))
                            message.blocks = [];
                        message.blocks.push($root.petajournal.Block.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetPageResponse
         * @function getTypeUrl
         * @memberof petajournal.GetPageResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetPageResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetPageResponse";
        };

        return GetPageResponse;
    })();

    petajournal.GetDocByNameRequest = (function() {

        /**
         * Properties of a GetDocByNameRequest.
         * @memberof petajournal
         * @interface IGetDocByNameRequest
         * @property {string|null} [title] GetDocByNameRequest title
         * @property {string|null} [dirPath] GetDocByNameRequest dirPath
         * @property {Array.<string>|null} [tags] GetDocByNameRequest tags
         */

        /**
         * Constructs a new GetDocByNameRequest.
         * @memberof petajournal
         * @classdesc Represents a GetDocByNameRequest.
         * @implements IGetDocByNameRequest
         * @constructor
         * @param {petajournal.IGetDocByNameRequest=} [properties] Properties to set
         */
        function GetDocByNameRequest(properties) {
            this.tags = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetDocByNameRequest title.
         * @member {string} title
         * @memberof petajournal.GetDocByNameRequest
         * @instance
         */
        GetDocByNameRequest.prototype.title = "";

        /**
         * GetDocByNameRequest dirPath.
         * @member {string} dirPath
         * @memberof petajournal.GetDocByNameRequest
         * @instance
         */
        GetDocByNameRequest.prototype.dirPath = "";

        /**
         * GetDocByNameRequest tags.
         * @member {Array.<string>} tags
         * @memberof petajournal.GetDocByNameRequest
         * @instance
         */
        GetDocByNameRequest.prototype.tags = $util.emptyArray;

        /**
         * Creates a new GetDocByNameRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetDocByNameRequest
         * @static
         * @param {petajournal.IGetDocByNameRequest=} [properties] Properties to set
         * @returns {petajournal.GetDocByNameRequest} GetDocByNameRequest instance
         */
        GetDocByNameRequest.create = function create(properties) {
            return new GetDocByNameRequest(properties);
        };

        /**
         * Encodes the specified GetDocByNameRequest message. Does not implicitly {@link petajournal.GetDocByNameRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetDocByNameRequest
         * @static
         * @param {petajournal.IGetDocByNameRequest} message GetDocByNameRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetDocByNameRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.title != null && Object.hasOwnProperty.call(message, "title"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.title);
            if (message.dirPath != null && Object.hasOwnProperty.call(message, "dirPath"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.dirPath);
            if (message.tags != null && message.tags.length)
                for (let i = 0; i < message.tags.length; ++i)
                    writer.uint32(/* id 3, wireType 2 =*/26).string(message.tags[i]);
            return writer;
        };

        /**
         * Decodes a GetDocByNameRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetDocByNameRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetDocByNameRequest} GetDocByNameRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetDocByNameRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetDocByNameRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.title = reader.string();
                        break;
                    }
                case 2: {
                        message.dirPath = reader.string();
                        break;
                    }
                case 3: {
                        if (!(message.tags && message.tags.length))
                            message.tags = [];
                        message.tags.push(reader.string());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetDocByNameRequest
         * @function getTypeUrl
         * @memberof petajournal.GetDocByNameRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetDocByNameRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetDocByNameRequest";
        };

        return GetDocByNameRequest;
    })();

    petajournal.GetDocByNameResponse = (function() {

        /**
         * Properties of a GetDocByNameResponse.
         * @memberof petajournal
         * @interface IGetDocByNameResponse
         * @property {petajournal.IPage|null} [page] GetDocByNameResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetDocByNameResponse blocks
         * @property {boolean|null} [created] GetDocByNameResponse created
         */

        /**
         * Constructs a new GetDocByNameResponse.
         * @memberof petajournal
         * @classdesc Represents a GetDocByNameResponse.
         * @implements IGetDocByNameResponse
         * @constructor
         * @param {petajournal.IGetDocByNameResponse=} [properties] Properties to set
         */
        function GetDocByNameResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetDocByNameResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetDocByNameResponse
         * @instance
         */
        GetDocByNameResponse.prototype.page = null;

        /**
         * GetDocByNameResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetDocByNameResponse
         * @instance
         */
        GetDocByNameResponse.prototype.blocks = $util.emptyArray;

        /**
         * GetDocByNameResponse created.
         * @member {boolean} created
         * @memberof petajournal.GetDocByNameResponse
         * @instance
         */
        GetDocByNameResponse.prototype.created = false;

        /**
         * Creates a new GetDocByNameResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetDocByNameResponse
         * @static
         * @param {petajournal.IGetDocByNameResponse=} [properties] Properties to set
         * @returns {petajournal.GetDocByNameResponse} GetDocByNameResponse instance
         */
        GetDocByNameResponse.create = function create(properties) {
            return new GetDocByNameResponse(properties);
        };

        /**
         * Encodes the specified GetDocByNameResponse message. Does not implicitly {@link petajournal.GetDocByNameResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetDocByNameResponse
         * @static
         * @param {petajournal.IGetDocByNameResponse} message GetDocByNameResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetDocByNameResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.page != null && Object.hasOwnProperty.call(message, "page"))
                $root.petajournal.Page.encode(message.page, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.blocks != null && message.blocks.length)
                for (let i = 0; i < message.blocks.length; ++i)
                    $root.petajournal.Block.encode(message.blocks[i], writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
            if (message.created != null && Object.hasOwnProperty.call(message, "created"))
                writer.uint32(/* id 3, wireType 0 =*/24).bool(message.created);
            return writer;
        };

        /**
         * Decodes a GetDocByNameResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetDocByNameResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetDocByNameResponse} GetDocByNameResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetDocByNameResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetDocByNameResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.page = $root.petajournal.Page.decode(reader, reader.uint32());
                        break;
                    }
                case 2: {
                        if (!(message.blocks && message.blocks.length))
                            message.blocks = [];
                        message.blocks.push($root.petajournal.Block.decode(reader, reader.uint32()));
                        break;
                    }
                case 3: {
                        message.created = reader.bool();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetDocByNameResponse
         * @function getTypeUrl
         * @memberof petajournal.GetDocByNameResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetDocByNameResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetDocByNameResponse";
        };

        return GetDocByNameResponse;
    })();

    petajournal.DeletePageRequest = (function() {

        /**
         * Properties of a DeletePageRequest.
         * @memberof petajournal
         * @interface IDeletePageRequest
         * @property {string|null} [pageId] DeletePageRequest pageId
         */

        /**
         * Constructs a new DeletePageRequest.
         * @memberof petajournal
         * @classdesc Represents a DeletePageRequest.
         * @implements IDeletePageRequest
         * @constructor
         * @param {petajournal.IDeletePageRequest=} [properties] Properties to set
         */
        function DeletePageRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * DeletePageRequest pageId.
         * @member {string} pageId
         * @memberof petajournal.DeletePageRequest
         * @instance
         */
        DeletePageRequest.prototype.pageId = "";

        /**
         * Creates a new DeletePageRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {petajournal.IDeletePageRequest=} [properties] Properties to set
         * @returns {petajournal.DeletePageRequest} DeletePageRequest instance
         */
        DeletePageRequest.create = function create(properties) {
            return new DeletePageRequest(properties);
        };

        /**
         * Encodes the specified DeletePageRequest message. Does not implicitly {@link petajournal.DeletePageRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {petajournal.IDeletePageRequest} message DeletePageRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeletePageRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            return writer;
        };

        /**
         * Decodes a DeletePageRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeletePageRequest} DeletePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeletePageRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeletePageRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for DeletePageRequest
         * @function getTypeUrl
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeletePageRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeletePageRequest";
        };

        return DeletePageRequest;
    })();

    petajournal.DeletePageResponse = (function() {

        /**
         * Properties of a DeletePageResponse.
         * @memberof petajournal
         * @interface IDeletePageResponse
         */

        /**
         * Constructs a new DeletePageResponse.
         * @memberof petajournal
         * @classdesc Represents a DeletePageResponse.
         * @implements IDeletePageResponse
         * @constructor
         * @param {petajournal.IDeletePageResponse=} [properties] Properties to set
         */
        function DeletePageResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Creates a new DeletePageResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {petajournal.IDeletePageResponse=} [properties] Properties to set
         * @returns {petajournal.DeletePageResponse} DeletePageResponse instance
         */
        DeletePageResponse.create = function create(properties) {
            return new DeletePageResponse(properties);
        };

        /**
         * Encodes the specified DeletePageResponse message. Does not implicitly {@link petajournal.DeletePageResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {petajournal.IDeletePageResponse} message DeletePageResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeletePageResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            return writer;
        };

        /**
         * Decodes a DeletePageResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeletePageResponse} DeletePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeletePageResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeletePageResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for DeletePageResponse
         * @function getTypeUrl
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeletePageResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeletePageResponse";
        };

        return DeletePageResponse;
    })();

    petajournal.UpdatePageRequest = (function() {

        /**
         * Properties of an UpdatePageRequest.
         * @memberof petajournal
         * @interface IUpdatePageRequest
         * @property {string|null} [pageId] UpdatePageRequest pageId
         * @property {string|null} [title] UpdatePageRequest title
         * @property {Array.<string>|null} [tags] UpdatePageRequest tags
         * @property {string|null} [propertiesJson] UpdatePageRequest propertiesJson
         */

        /**
         * Constructs a new UpdatePageRequest.
         * @memberof petajournal
         * @classdesc Represents an UpdatePageRequest.
         * @implements IUpdatePageRequest
         * @constructor
         * @param {petajournal.IUpdatePageRequest=} [properties] Properties to set
         */
        function UpdatePageRequest(properties) {
            this.tags = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdatePageRequest pageId.
         * @member {string} pageId
         * @memberof petajournal.UpdatePageRequest
         * @instance
         */
        UpdatePageRequest.prototype.pageId = "";

        /**
         * UpdatePageRequest title.
         * @member {string|null|undefined} title
         * @memberof petajournal.UpdatePageRequest
         * @instance
         */
        UpdatePageRequest.prototype.title = null;

        /**
         * UpdatePageRequest tags.
         * @member {Array.<string>} tags
         * @memberof petajournal.UpdatePageRequest
         * @instance
         */
        UpdatePageRequest.prototype.tags = $util.emptyArray;

        /**
         * UpdatePageRequest propertiesJson.
         * @member {string} propertiesJson
         * @memberof petajournal.UpdatePageRequest
         * @instance
         */
        UpdatePageRequest.prototype.propertiesJson = "";

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdatePageRequest.prototype, "_title", {
            get: $util.oneOfGetter($oneOfFields = ["title"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new UpdatePageRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {petajournal.IUpdatePageRequest=} [properties] Properties to set
         * @returns {petajournal.UpdatePageRequest} UpdatePageRequest instance
         */
        UpdatePageRequest.create = function create(properties) {
            return new UpdatePageRequest(properties);
        };

        /**
         * Encodes the specified UpdatePageRequest message. Does not implicitly {@link petajournal.UpdatePageRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {petajournal.IUpdatePageRequest} message UpdatePageRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdatePageRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            if (message.title != null && Object.hasOwnProperty.call(message, "title"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.title);
            if (message.tags != null && message.tags.length)
                for (let i = 0; i < message.tags.length; ++i)
                    writer.uint32(/* id 3, wireType 2 =*/26).string(message.tags[i]);
            if (message.propertiesJson != null && Object.hasOwnProperty.call(message, "propertiesJson"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.propertiesJson);
            return writer;
        };

        /**
         * Decodes an UpdatePageRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdatePageRequest} UpdatePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdatePageRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdatePageRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                case 2: {
                        message.title = reader.string();
                        break;
                    }
                case 3: {
                        if (!(message.tags && message.tags.length))
                            message.tags = [];
                        message.tags.push(reader.string());
                        break;
                    }
                case 4: {
                        message.propertiesJson = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdatePageRequest
         * @function getTypeUrl
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdatePageRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdatePageRequest";
        };

        return UpdatePageRequest;
    })();

    petajournal.UpdatePageResponse = (function() {

        /**
         * Properties of an UpdatePageResponse.
         * @memberof petajournal
         * @interface IUpdatePageResponse
         * @property {petajournal.IPage|null} [page] UpdatePageResponse page
         */

        /**
         * Constructs a new UpdatePageResponse.
         * @memberof petajournal
         * @classdesc Represents an UpdatePageResponse.
         * @implements IUpdatePageResponse
         * @constructor
         * @param {petajournal.IUpdatePageResponse=} [properties] Properties to set
         */
        function UpdatePageResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdatePageResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.UpdatePageResponse
         * @instance
         */
        UpdatePageResponse.prototype.page = null;

        /**
         * Creates a new UpdatePageResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {petajournal.IUpdatePageResponse=} [properties] Properties to set
         * @returns {petajournal.UpdatePageResponse} UpdatePageResponse instance
         */
        UpdatePageResponse.create = function create(properties) {
            return new UpdatePageResponse(properties);
        };

        /**
         * Encodes the specified UpdatePageResponse message. Does not implicitly {@link petajournal.UpdatePageResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {petajournal.IUpdatePageResponse} message UpdatePageResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdatePageResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.page != null && Object.hasOwnProperty.call(message, "page"))
                $root.petajournal.Page.encode(message.page, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes an UpdatePageResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdatePageResponse} UpdatePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdatePageResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdatePageResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.page = $root.petajournal.Page.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdatePageResponse
         * @function getTypeUrl
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdatePageResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdatePageResponse";
        };

        return UpdatePageResponse;
    })();

    petajournal.CreateBlockRequest = (function() {

        /**
         * Properties of a CreateBlockRequest.
         * @memberof petajournal
         * @interface ICreateBlockRequest
         * @property {string|null} [pageId] CreateBlockRequest pageId
         * @property {string|null} [afterBlockId] CreateBlockRequest afterBlockId
         * @property {number|null} [level] CreateBlockRequest level
         * @property {string|null} [body] CreateBlockRequest body
         */

        /**
         * Constructs a new CreateBlockRequest.
         * @memberof petajournal
         * @classdesc Represents a CreateBlockRequest.
         * @implements ICreateBlockRequest
         * @constructor
         * @param {petajournal.ICreateBlockRequest=} [properties] Properties to set
         */
        function CreateBlockRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * CreateBlockRequest pageId.
         * @member {string} pageId
         * @memberof petajournal.CreateBlockRequest
         * @instance
         */
        CreateBlockRequest.prototype.pageId = "";

        /**
         * CreateBlockRequest afterBlockId.
         * @member {string} afterBlockId
         * @memberof petajournal.CreateBlockRequest
         * @instance
         */
        CreateBlockRequest.prototype.afterBlockId = "";

        /**
         * CreateBlockRequest level.
         * @member {number} level
         * @memberof petajournal.CreateBlockRequest
         * @instance
         */
        CreateBlockRequest.prototype.level = 0;

        /**
         * CreateBlockRequest body.
         * @member {string} body
         * @memberof petajournal.CreateBlockRequest
         * @instance
         */
        CreateBlockRequest.prototype.body = "";

        /**
         * Creates a new CreateBlockRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {petajournal.ICreateBlockRequest=} [properties] Properties to set
         * @returns {petajournal.CreateBlockRequest} CreateBlockRequest instance
         */
        CreateBlockRequest.create = function create(properties) {
            return new CreateBlockRequest(properties);
        };

        /**
         * Encodes the specified CreateBlockRequest message. Does not implicitly {@link petajournal.CreateBlockRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {petajournal.ICreateBlockRequest} message CreateBlockRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBlockRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            if (message.afterBlockId != null && Object.hasOwnProperty.call(message, "afterBlockId"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.afterBlockId);
            if (message.level != null && Object.hasOwnProperty.call(message, "level"))
                writer.uint32(/* id 3, wireType 0 =*/24).int32(message.level);
            if (message.body != null && Object.hasOwnProperty.call(message, "body"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.body);
            return writer;
        };

        /**
         * Decodes a CreateBlockRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.CreateBlockRequest} CreateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBlockRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.CreateBlockRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                case 2: {
                        message.afterBlockId = reader.string();
                        break;
                    }
                case 3: {
                        message.level = reader.int32();
                        break;
                    }
                case 4: {
                        message.body = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for CreateBlockRequest
         * @function getTypeUrl
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        CreateBlockRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.CreateBlockRequest";
        };

        return CreateBlockRequest;
    })();

    petajournal.CreateBlockResponse = (function() {

        /**
         * Properties of a CreateBlockResponse.
         * @memberof petajournal
         * @interface ICreateBlockResponse
         * @property {petajournal.IBlock|null} [block] CreateBlockResponse block
         */

        /**
         * Constructs a new CreateBlockResponse.
         * @memberof petajournal
         * @classdesc Represents a CreateBlockResponse.
         * @implements ICreateBlockResponse
         * @constructor
         * @param {petajournal.ICreateBlockResponse=} [properties] Properties to set
         */
        function CreateBlockResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * CreateBlockResponse block.
         * @member {petajournal.IBlock|null|undefined} block
         * @memberof petajournal.CreateBlockResponse
         * @instance
         */
        CreateBlockResponse.prototype.block = null;

        /**
         * Creates a new CreateBlockResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {petajournal.ICreateBlockResponse=} [properties] Properties to set
         * @returns {petajournal.CreateBlockResponse} CreateBlockResponse instance
         */
        CreateBlockResponse.create = function create(properties) {
            return new CreateBlockResponse(properties);
        };

        /**
         * Encodes the specified CreateBlockResponse message. Does not implicitly {@link petajournal.CreateBlockResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {petajournal.ICreateBlockResponse} message CreateBlockResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBlockResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.block != null && Object.hasOwnProperty.call(message, "block"))
                $root.petajournal.Block.encode(message.block, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a CreateBlockResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.CreateBlockResponse} CreateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBlockResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.CreateBlockResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.block = $root.petajournal.Block.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for CreateBlockResponse
         * @function getTypeUrl
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        CreateBlockResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.CreateBlockResponse";
        };

        return CreateBlockResponse;
    })();

    petajournal.UpdateBlockRequest = (function() {

        /**
         * Properties of an UpdateBlockRequest.
         * @memberof petajournal
         * @interface IUpdateBlockRequest
         * @property {string|null} [blockId] UpdateBlockRequest blockId
         * @property {string|null} [body] UpdateBlockRequest body
         * @property {number|null} [level] UpdateBlockRequest level
         * @property {boolean|null} [collapsed] UpdateBlockRequest collapsed
         * @property {string|null} [status] UpdateBlockRequest status
         * @property {string|null} [propertiesJson] UpdateBlockRequest propertiesJson
         */

        /**
         * Constructs a new UpdateBlockRequest.
         * @memberof petajournal
         * @classdesc Represents an UpdateBlockRequest.
         * @implements IUpdateBlockRequest
         * @constructor
         * @param {petajournal.IUpdateBlockRequest=} [properties] Properties to set
         */
        function UpdateBlockRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateBlockRequest blockId.
         * @member {string} blockId
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         */
        UpdateBlockRequest.prototype.blockId = "";

        /**
         * UpdateBlockRequest body.
         * @member {string|null|undefined} body
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         */
        UpdateBlockRequest.prototype.body = null;

        /**
         * UpdateBlockRequest level.
         * @member {number|null|undefined} level
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         */
        UpdateBlockRequest.prototype.level = null;

        /**
         * UpdateBlockRequest collapsed.
         * @member {boolean|null|undefined} collapsed
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         */
        UpdateBlockRequest.prototype.collapsed = null;

        /**
         * UpdateBlockRequest status.
         * @member {string|null|undefined} status
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         */
        UpdateBlockRequest.prototype.status = null;

        /**
         * UpdateBlockRequest propertiesJson.
         * @member {string|null|undefined} propertiesJson
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         */
        UpdateBlockRequest.prototype.propertiesJson = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdateBlockRequest.prototype, "_body", {
            get: $util.oneOfGetter($oneOfFields = ["body"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdateBlockRequest.prototype, "_level", {
            get: $util.oneOfGetter($oneOfFields = ["level"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdateBlockRequest.prototype, "_collapsed", {
            get: $util.oneOfGetter($oneOfFields = ["collapsed"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdateBlockRequest.prototype, "_status", {
            get: $util.oneOfGetter($oneOfFields = ["status"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdateBlockRequest.prototype, "_propertiesJson", {
            get: $util.oneOfGetter($oneOfFields = ["propertiesJson"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new UpdateBlockRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {petajournal.IUpdateBlockRequest=} [properties] Properties to set
         * @returns {petajournal.UpdateBlockRequest} UpdateBlockRequest instance
         */
        UpdateBlockRequest.create = function create(properties) {
            return new UpdateBlockRequest(properties);
        };

        /**
         * Encodes the specified UpdateBlockRequest message. Does not implicitly {@link petajournal.UpdateBlockRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {petajournal.IUpdateBlockRequest} message UpdateBlockRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blockId != null && Object.hasOwnProperty.call(message, "blockId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.blockId);
            if (message.body != null && Object.hasOwnProperty.call(message, "body"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.body);
            if (message.level != null && Object.hasOwnProperty.call(message, "level"))
                writer.uint32(/* id 3, wireType 0 =*/24).int32(message.level);
            if (message.collapsed != null && Object.hasOwnProperty.call(message, "collapsed"))
                writer.uint32(/* id 4, wireType 0 =*/32).bool(message.collapsed);
            if (message.status != null && Object.hasOwnProperty.call(message, "status"))
                writer.uint32(/* id 5, wireType 2 =*/42).string(message.status);
            if (message.propertiesJson != null && Object.hasOwnProperty.call(message, "propertiesJson"))
                writer.uint32(/* id 6, wireType 2 =*/50).string(message.propertiesJson);
            return writer;
        };

        /**
         * Decodes an UpdateBlockRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateBlockRequest} UpdateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateBlockRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.blockId = reader.string();
                        break;
                    }
                case 2: {
                        message.body = reader.string();
                        break;
                    }
                case 3: {
                        message.level = reader.int32();
                        break;
                    }
                case 4: {
                        message.collapsed = reader.bool();
                        break;
                    }
                case 5: {
                        message.status = reader.string();
                        break;
                    }
                case 6: {
                        message.propertiesJson = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdateBlockRequest
         * @function getTypeUrl
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateBlockRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateBlockRequest";
        };

        return UpdateBlockRequest;
    })();

    petajournal.UpdateBlockResponse = (function() {

        /**
         * Properties of an UpdateBlockResponse.
         * @memberof petajournal
         * @interface IUpdateBlockResponse
         * @property {petajournal.IBlock|null} [block] UpdateBlockResponse block
         */

        /**
         * Constructs a new UpdateBlockResponse.
         * @memberof petajournal
         * @classdesc Represents an UpdateBlockResponse.
         * @implements IUpdateBlockResponse
         * @constructor
         * @param {petajournal.IUpdateBlockResponse=} [properties] Properties to set
         */
        function UpdateBlockResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateBlockResponse block.
         * @member {petajournal.IBlock|null|undefined} block
         * @memberof petajournal.UpdateBlockResponse
         * @instance
         */
        UpdateBlockResponse.prototype.block = null;

        /**
         * Creates a new UpdateBlockResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {petajournal.IUpdateBlockResponse=} [properties] Properties to set
         * @returns {petajournal.UpdateBlockResponse} UpdateBlockResponse instance
         */
        UpdateBlockResponse.create = function create(properties) {
            return new UpdateBlockResponse(properties);
        };

        /**
         * Encodes the specified UpdateBlockResponse message. Does not implicitly {@link petajournal.UpdateBlockResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {petajournal.IUpdateBlockResponse} message UpdateBlockResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.block != null && Object.hasOwnProperty.call(message, "block"))
                $root.petajournal.Block.encode(message.block, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes an UpdateBlockResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateBlockResponse} UpdateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateBlockResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.block = $root.petajournal.Block.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdateBlockResponse
         * @function getTypeUrl
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateBlockResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateBlockResponse";
        };

        return UpdateBlockResponse;
    })();

    petajournal.MoveBlocksRequest = (function() {

        /**
         * Properties of a MoveBlocksRequest.
         * @memberof petajournal
         * @interface IMoveBlocksRequest
         * @property {Array.<string>|null} [blockIds] MoveBlocksRequest blockIds
         * @property {string|null} [targetPageId] MoveBlocksRequest targetPageId
         * @property {string|null} [afterBlockId] MoveBlocksRequest afterBlockId
         * @property {number|null} [levelDelta] MoveBlocksRequest levelDelta
         */

        /**
         * Constructs a new MoveBlocksRequest.
         * @memberof petajournal
         * @classdesc Represents a MoveBlocksRequest.
         * @implements IMoveBlocksRequest
         * @constructor
         * @param {petajournal.IMoveBlocksRequest=} [properties] Properties to set
         */
        function MoveBlocksRequest(properties) {
            this.blockIds = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * MoveBlocksRequest blockIds.
         * @member {Array.<string>} blockIds
         * @memberof petajournal.MoveBlocksRequest
         * @instance
         */
        MoveBlocksRequest.prototype.blockIds = $util.emptyArray;

        /**
         * MoveBlocksRequest targetPageId.
         * @member {string} targetPageId
         * @memberof petajournal.MoveBlocksRequest
         * @instance
         */
        MoveBlocksRequest.prototype.targetPageId = "";

        /**
         * MoveBlocksRequest afterBlockId.
         * @member {string} afterBlockId
         * @memberof petajournal.MoveBlocksRequest
         * @instance
         */
        MoveBlocksRequest.prototype.afterBlockId = "";

        /**
         * MoveBlocksRequest levelDelta.
         * @member {number|null|undefined} levelDelta
         * @memberof petajournal.MoveBlocksRequest
         * @instance
         */
        MoveBlocksRequest.prototype.levelDelta = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(MoveBlocksRequest.prototype, "_levelDelta", {
            get: $util.oneOfGetter($oneOfFields = ["levelDelta"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new MoveBlocksRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {petajournal.IMoveBlocksRequest=} [properties] Properties to set
         * @returns {petajournal.MoveBlocksRequest} MoveBlocksRequest instance
         */
        MoveBlocksRequest.create = function create(properties) {
            return new MoveBlocksRequest(properties);
        };

        /**
         * Encodes the specified MoveBlocksRequest message. Does not implicitly {@link petajournal.MoveBlocksRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {petajournal.IMoveBlocksRequest} message MoveBlocksRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBlocksRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blockIds != null && message.blockIds.length)
                for (let i = 0; i < message.blockIds.length; ++i)
                    writer.uint32(/* id 1, wireType 2 =*/10).string(message.blockIds[i]);
            if (message.targetPageId != null && Object.hasOwnProperty.call(message, "targetPageId"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.targetPageId);
            if (message.afterBlockId != null && Object.hasOwnProperty.call(message, "afterBlockId"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.afterBlockId);
            if (message.levelDelta != null && Object.hasOwnProperty.call(message, "levelDelta"))
                writer.uint32(/* id 4, wireType 0 =*/32).int32(message.levelDelta);
            return writer;
        };

        /**
         * Decodes a MoveBlocksRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.MoveBlocksRequest} MoveBlocksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBlocksRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.MoveBlocksRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.blockIds && message.blockIds.length))
                            message.blockIds = [];
                        message.blockIds.push(reader.string());
                        break;
                    }
                case 2: {
                        message.targetPageId = reader.string();
                        break;
                    }
                case 3: {
                        message.afterBlockId = reader.string();
                        break;
                    }
                case 4: {
                        message.levelDelta = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for MoveBlocksRequest
         * @function getTypeUrl
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        MoveBlocksRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.MoveBlocksRequest";
        };

        return MoveBlocksRequest;
    })();

    petajournal.MoveBlocksResponse = (function() {

        /**
         * Properties of a MoveBlocksResponse.
         * @memberof petajournal
         * @interface IMoveBlocksResponse
         * @property {Array.<petajournal.IBlock>|null} [blocks] MoveBlocksResponse blocks
         */

        /**
         * Constructs a new MoveBlocksResponse.
         * @memberof petajournal
         * @classdesc Represents a MoveBlocksResponse.
         * @implements IMoveBlocksResponse
         * @constructor
         * @param {petajournal.IMoveBlocksResponse=} [properties] Properties to set
         */
        function MoveBlocksResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * MoveBlocksResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.MoveBlocksResponse
         * @instance
         */
        MoveBlocksResponse.prototype.blocks = $util.emptyArray;

        /**
         * Creates a new MoveBlocksResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {petajournal.IMoveBlocksResponse=} [properties] Properties to set
         * @returns {petajournal.MoveBlocksResponse} MoveBlocksResponse instance
         */
        MoveBlocksResponse.create = function create(properties) {
            return new MoveBlocksResponse(properties);
        };

        /**
         * Encodes the specified MoveBlocksResponse message. Does not implicitly {@link petajournal.MoveBlocksResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {petajournal.IMoveBlocksResponse} message MoveBlocksResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBlocksResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blocks != null && message.blocks.length)
                for (let i = 0; i < message.blocks.length; ++i)
                    $root.petajournal.Block.encode(message.blocks[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a MoveBlocksResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.MoveBlocksResponse} MoveBlocksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBlocksResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.MoveBlocksResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.blocks && message.blocks.length))
                            message.blocks = [];
                        message.blocks.push($root.petajournal.Block.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for MoveBlocksResponse
         * @function getTypeUrl
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        MoveBlocksResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.MoveBlocksResponse";
        };

        return MoveBlocksResponse;
    })();

    petajournal.DeleteBlockRequest = (function() {

        /**
         * Properties of a DeleteBlockRequest.
         * @memberof petajournal
         * @interface IDeleteBlockRequest
         * @property {string|null} [blockId] DeleteBlockRequest blockId
         */

        /**
         * Constructs a new DeleteBlockRequest.
         * @memberof petajournal
         * @classdesc Represents a DeleteBlockRequest.
         * @implements IDeleteBlockRequest
         * @constructor
         * @param {petajournal.IDeleteBlockRequest=} [properties] Properties to set
         */
        function DeleteBlockRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * DeleteBlockRequest blockId.
         * @member {string} blockId
         * @memberof petajournal.DeleteBlockRequest
         * @instance
         */
        DeleteBlockRequest.prototype.blockId = "";

        /**
         * Creates a new DeleteBlockRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {petajournal.IDeleteBlockRequest=} [properties] Properties to set
         * @returns {petajournal.DeleteBlockRequest} DeleteBlockRequest instance
         */
        DeleteBlockRequest.create = function create(properties) {
            return new DeleteBlockRequest(properties);
        };

        /**
         * Encodes the specified DeleteBlockRequest message. Does not implicitly {@link petajournal.DeleteBlockRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {petajournal.IDeleteBlockRequest} message DeleteBlockRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBlockRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blockId != null && Object.hasOwnProperty.call(message, "blockId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.blockId);
            return writer;
        };

        /**
         * Decodes a DeleteBlockRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeleteBlockRequest} DeleteBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBlockRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeleteBlockRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.blockId = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for DeleteBlockRequest
         * @function getTypeUrl
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeleteBlockRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeleteBlockRequest";
        };

        return DeleteBlockRequest;
    })();

    petajournal.DeleteBlockResponse = (function() {

        /**
         * Properties of a DeleteBlockResponse.
         * @memberof petajournal
         * @interface IDeleteBlockResponse
         */

        /**
         * Constructs a new DeleteBlockResponse.
         * @memberof petajournal
         * @classdesc Represents a DeleteBlockResponse.
         * @implements IDeleteBlockResponse
         * @constructor
         * @param {petajournal.IDeleteBlockResponse=} [properties] Properties to set
         */
        function DeleteBlockResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Creates a new DeleteBlockResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {petajournal.IDeleteBlockResponse=} [properties] Properties to set
         * @returns {petajournal.DeleteBlockResponse} DeleteBlockResponse instance
         */
        DeleteBlockResponse.create = function create(properties) {
            return new DeleteBlockResponse(properties);
        };

        /**
         * Encodes the specified DeleteBlockResponse message. Does not implicitly {@link petajournal.DeleteBlockResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {petajournal.IDeleteBlockResponse} message DeleteBlockResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBlockResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            return writer;
        };

        /**
         * Decodes a DeleteBlockResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeleteBlockResponse} DeleteBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBlockResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeleteBlockResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for DeleteBlockResponse
         * @function getTypeUrl
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeleteBlockResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeleteBlockResponse";
        };

        return DeleteBlockResponse;
    })();

    petajournal.UpdateBlockStatusRequest = (function() {

        /**
         * Properties of an UpdateBlockStatusRequest.
         * @memberof petajournal
         * @interface IUpdateBlockStatusRequest
         * @property {string|null} [blockId] UpdateBlockStatusRequest blockId
         * @property {string|null} [status] UpdateBlockStatusRequest status
         */

        /**
         * Constructs a new UpdateBlockStatusRequest.
         * @memberof petajournal
         * @classdesc Represents an UpdateBlockStatusRequest.
         * @implements IUpdateBlockStatusRequest
         * @constructor
         * @param {petajournal.IUpdateBlockStatusRequest=} [properties] Properties to set
         */
        function UpdateBlockStatusRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateBlockStatusRequest blockId.
         * @member {string} blockId
         * @memberof petajournal.UpdateBlockStatusRequest
         * @instance
         */
        UpdateBlockStatusRequest.prototype.blockId = "";

        /**
         * UpdateBlockStatusRequest status.
         * @member {string} status
         * @memberof petajournal.UpdateBlockStatusRequest
         * @instance
         */
        UpdateBlockStatusRequest.prototype.status = "";

        /**
         * Creates a new UpdateBlockStatusRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {petajournal.IUpdateBlockStatusRequest=} [properties] Properties to set
         * @returns {petajournal.UpdateBlockStatusRequest} UpdateBlockStatusRequest instance
         */
        UpdateBlockStatusRequest.create = function create(properties) {
            return new UpdateBlockStatusRequest(properties);
        };

        /**
         * Encodes the specified UpdateBlockStatusRequest message. Does not implicitly {@link petajournal.UpdateBlockStatusRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {petajournal.IUpdateBlockStatusRequest} message UpdateBlockStatusRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockStatusRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blockId != null && Object.hasOwnProperty.call(message, "blockId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.blockId);
            if (message.status != null && Object.hasOwnProperty.call(message, "status"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.status);
            return writer;
        };

        /**
         * Decodes an UpdateBlockStatusRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateBlockStatusRequest} UpdateBlockStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockStatusRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateBlockStatusRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.blockId = reader.string();
                        break;
                    }
                case 2: {
                        message.status = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdateBlockStatusRequest
         * @function getTypeUrl
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateBlockStatusRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateBlockStatusRequest";
        };

        return UpdateBlockStatusRequest;
    })();

    petajournal.UpdateBlockStatusResponse = (function() {

        /**
         * Properties of an UpdateBlockStatusResponse.
         * @memberof petajournal
         * @interface IUpdateBlockStatusResponse
         * @property {petajournal.IBlock|null} [block] UpdateBlockStatusResponse block
         */

        /**
         * Constructs a new UpdateBlockStatusResponse.
         * @memberof petajournal
         * @classdesc Represents an UpdateBlockStatusResponse.
         * @implements IUpdateBlockStatusResponse
         * @constructor
         * @param {petajournal.IUpdateBlockStatusResponse=} [properties] Properties to set
         */
        function UpdateBlockStatusResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateBlockStatusResponse block.
         * @member {petajournal.IBlock|null|undefined} block
         * @memberof petajournal.UpdateBlockStatusResponse
         * @instance
         */
        UpdateBlockStatusResponse.prototype.block = null;

        /**
         * Creates a new UpdateBlockStatusResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {petajournal.IUpdateBlockStatusResponse=} [properties] Properties to set
         * @returns {petajournal.UpdateBlockStatusResponse} UpdateBlockStatusResponse instance
         */
        UpdateBlockStatusResponse.create = function create(properties) {
            return new UpdateBlockStatusResponse(properties);
        };

        /**
         * Encodes the specified UpdateBlockStatusResponse message. Does not implicitly {@link petajournal.UpdateBlockStatusResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {petajournal.IUpdateBlockStatusResponse} message UpdateBlockStatusResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockStatusResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.block != null && Object.hasOwnProperty.call(message, "block"))
                $root.petajournal.Block.encode(message.block, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes an UpdateBlockStatusResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateBlockStatusResponse} UpdateBlockStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockStatusResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateBlockStatusResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.block = $root.petajournal.Block.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdateBlockStatusResponse
         * @function getTypeUrl
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateBlockStatusResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateBlockStatusResponse";
        };

        return UpdateBlockStatusResponse;
    })();

    petajournal.SearchPageNamesRequest = (function() {

        /**
         * Properties of a SearchPageNamesRequest.
         * @memberof petajournal
         * @interface ISearchPageNamesRequest
         * @property {string|null} [query] SearchPageNamesRequest query
         * @property {number|null} [maxResults] SearchPageNamesRequest maxResults
         */

        /**
         * Constructs a new SearchPageNamesRequest.
         * @memberof petajournal
         * @classdesc Represents a SearchPageNamesRequest.
         * @implements ISearchPageNamesRequest
         * @constructor
         * @param {petajournal.ISearchPageNamesRequest=} [properties] Properties to set
         */
        function SearchPageNamesRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchPageNamesRequest query.
         * @member {string} query
         * @memberof petajournal.SearchPageNamesRequest
         * @instance
         */
        SearchPageNamesRequest.prototype.query = "";

        /**
         * SearchPageNamesRequest maxResults.
         * @member {number} maxResults
         * @memberof petajournal.SearchPageNamesRequest
         * @instance
         */
        SearchPageNamesRequest.prototype.maxResults = 0;

        /**
         * Creates a new SearchPageNamesRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {petajournal.ISearchPageNamesRequest=} [properties] Properties to set
         * @returns {petajournal.SearchPageNamesRequest} SearchPageNamesRequest instance
         */
        SearchPageNamesRequest.create = function create(properties) {
            return new SearchPageNamesRequest(properties);
        };

        /**
         * Encodes the specified SearchPageNamesRequest message. Does not implicitly {@link petajournal.SearchPageNamesRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {petajournal.ISearchPageNamesRequest} message SearchPageNamesRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchPageNamesRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.query != null && Object.hasOwnProperty.call(message, "query"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.query);
            if (message.maxResults != null && Object.hasOwnProperty.call(message, "maxResults"))
                writer.uint32(/* id 2, wireType 0 =*/16).int32(message.maxResults);
            return writer;
        };

        /**
         * Decodes a SearchPageNamesRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchPageNamesRequest} SearchPageNamesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchPageNamesRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchPageNamesRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.query = reader.string();
                        break;
                    }
                case 2: {
                        message.maxResults = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchPageNamesRequest
         * @function getTypeUrl
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchPageNamesRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchPageNamesRequest";
        };

        return SearchPageNamesRequest;
    })();

    petajournal.SearchPageNamesResponse = (function() {

        /**
         * Properties of a SearchPageNamesResponse.
         * @memberof petajournal
         * @interface ISearchPageNamesResponse
         * @property {Array.<petajournal.ISearchResult>|null} [results] SearchPageNamesResponse results
         */

        /**
         * Constructs a new SearchPageNamesResponse.
         * @memberof petajournal
         * @classdesc Represents a SearchPageNamesResponse.
         * @implements ISearchPageNamesResponse
         * @constructor
         * @param {petajournal.ISearchPageNamesResponse=} [properties] Properties to set
         */
        function SearchPageNamesResponse(properties) {
            this.results = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchPageNamesResponse results.
         * @member {Array.<petajournal.ISearchResult>} results
         * @memberof petajournal.SearchPageNamesResponse
         * @instance
         */
        SearchPageNamesResponse.prototype.results = $util.emptyArray;

        /**
         * Creates a new SearchPageNamesResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {petajournal.ISearchPageNamesResponse=} [properties] Properties to set
         * @returns {petajournal.SearchPageNamesResponse} SearchPageNamesResponse instance
         */
        SearchPageNamesResponse.create = function create(properties) {
            return new SearchPageNamesResponse(properties);
        };

        /**
         * Encodes the specified SearchPageNamesResponse message. Does not implicitly {@link petajournal.SearchPageNamesResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {petajournal.ISearchPageNamesResponse} message SearchPageNamesResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchPageNamesResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.results != null && message.results.length)
                for (let i = 0; i < message.results.length; ++i)
                    $root.petajournal.SearchResult.encode(message.results[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a SearchPageNamesResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchPageNamesResponse} SearchPageNamesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchPageNamesResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchPageNamesResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.results && message.results.length))
                            message.results = [];
                        message.results.push($root.petajournal.SearchResult.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchPageNamesResponse
         * @function getTypeUrl
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchPageNamesResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchPageNamesResponse";
        };

        return SearchPageNamesResponse;
    })();

    petajournal.SearchFullTextRequest = (function() {

        /**
         * Properties of a SearchFullTextRequest.
         * @memberof petajournal
         * @interface ISearchFullTextRequest
         * @property {string|null} [query] SearchFullTextRequest query
         * @property {number|null} [maxResults] SearchFullTextRequest maxResults
         */

        /**
         * Constructs a new SearchFullTextRequest.
         * @memberof petajournal
         * @classdesc Represents a SearchFullTextRequest.
         * @implements ISearchFullTextRequest
         * @constructor
         * @param {petajournal.ISearchFullTextRequest=} [properties] Properties to set
         */
        function SearchFullTextRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchFullTextRequest query.
         * @member {string} query
         * @memberof petajournal.SearchFullTextRequest
         * @instance
         */
        SearchFullTextRequest.prototype.query = "";

        /**
         * SearchFullTextRequest maxResults.
         * @member {number} maxResults
         * @memberof petajournal.SearchFullTextRequest
         * @instance
         */
        SearchFullTextRequest.prototype.maxResults = 0;

        /**
         * Creates a new SearchFullTextRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {petajournal.ISearchFullTextRequest=} [properties] Properties to set
         * @returns {petajournal.SearchFullTextRequest} SearchFullTextRequest instance
         */
        SearchFullTextRequest.create = function create(properties) {
            return new SearchFullTextRequest(properties);
        };

        /**
         * Encodes the specified SearchFullTextRequest message. Does not implicitly {@link petajournal.SearchFullTextRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {petajournal.ISearchFullTextRequest} message SearchFullTextRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchFullTextRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.query != null && Object.hasOwnProperty.call(message, "query"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.query);
            if (message.maxResults != null && Object.hasOwnProperty.call(message, "maxResults"))
                writer.uint32(/* id 2, wireType 0 =*/16).int32(message.maxResults);
            return writer;
        };

        /**
         * Decodes a SearchFullTextRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchFullTextRequest} SearchFullTextRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchFullTextRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchFullTextRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.query = reader.string();
                        break;
                    }
                case 2: {
                        message.maxResults = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchFullTextRequest
         * @function getTypeUrl
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchFullTextRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchFullTextRequest";
        };

        return SearchFullTextRequest;
    })();

    petajournal.SearchFullTextResponse = (function() {

        /**
         * Properties of a SearchFullTextResponse.
         * @memberof petajournal
         * @interface ISearchFullTextResponse
         * @property {Array.<petajournal.ISearchResult>|null} [results] SearchFullTextResponse results
         * @property {number|null} [totalHits] SearchFullTextResponse totalHits
         */

        /**
         * Constructs a new SearchFullTextResponse.
         * @memberof petajournal
         * @classdesc Represents a SearchFullTextResponse.
         * @implements ISearchFullTextResponse
         * @constructor
         * @param {petajournal.ISearchFullTextResponse=} [properties] Properties to set
         */
        function SearchFullTextResponse(properties) {
            this.results = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SearchFullTextResponse results.
         * @member {Array.<petajournal.ISearchResult>} results
         * @memberof petajournal.SearchFullTextResponse
         * @instance
         */
        SearchFullTextResponse.prototype.results = $util.emptyArray;

        /**
         * SearchFullTextResponse totalHits.
         * @member {number} totalHits
         * @memberof petajournal.SearchFullTextResponse
         * @instance
         */
        SearchFullTextResponse.prototype.totalHits = 0;

        /**
         * Creates a new SearchFullTextResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {petajournal.ISearchFullTextResponse=} [properties] Properties to set
         * @returns {petajournal.SearchFullTextResponse} SearchFullTextResponse instance
         */
        SearchFullTextResponse.create = function create(properties) {
            return new SearchFullTextResponse(properties);
        };

        /**
         * Encodes the specified SearchFullTextResponse message. Does not implicitly {@link petajournal.SearchFullTextResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {petajournal.ISearchFullTextResponse} message SearchFullTextResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchFullTextResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.results != null && message.results.length)
                for (let i = 0; i < message.results.length; ++i)
                    $root.petajournal.SearchResult.encode(message.results[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.totalHits != null && Object.hasOwnProperty.call(message, "totalHits"))
                writer.uint32(/* id 2, wireType 0 =*/16).int32(message.totalHits);
            return writer;
        };

        /**
         * Decodes a SearchFullTextResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.SearchFullTextResponse} SearchFullTextResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchFullTextResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.SearchFullTextResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.results && message.results.length))
                            message.results = [];
                        message.results.push($root.petajournal.SearchResult.decode(reader, reader.uint32()));
                        break;
                    }
                case 2: {
                        message.totalHits = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for SearchFullTextResponse
         * @function getTypeUrl
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SearchFullTextResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.SearchFullTextResponse";
        };

        return SearchFullTextResponse;
    })();

    petajournal.GetRecentPagesRequest = (function() {

        /**
         * Properties of a GetRecentPagesRequest.
         * @memberof petajournal
         * @interface IGetRecentPagesRequest
         * @property {number|null} [maxResults] GetRecentPagesRequest maxResults
         */

        /**
         * Constructs a new GetRecentPagesRequest.
         * @memberof petajournal
         * @classdesc Represents a GetRecentPagesRequest.
         * @implements IGetRecentPagesRequest
         * @constructor
         * @param {petajournal.IGetRecentPagesRequest=} [properties] Properties to set
         */
        function GetRecentPagesRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetRecentPagesRequest maxResults.
         * @member {number} maxResults
         * @memberof petajournal.GetRecentPagesRequest
         * @instance
         */
        GetRecentPagesRequest.prototype.maxResults = 0;

        /**
         * Creates a new GetRecentPagesRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {petajournal.IGetRecentPagesRequest=} [properties] Properties to set
         * @returns {petajournal.GetRecentPagesRequest} GetRecentPagesRequest instance
         */
        GetRecentPagesRequest.create = function create(properties) {
            return new GetRecentPagesRequest(properties);
        };

        /**
         * Encodes the specified GetRecentPagesRequest message. Does not implicitly {@link petajournal.GetRecentPagesRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {petajournal.IGetRecentPagesRequest} message GetRecentPagesRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetRecentPagesRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.maxResults != null && Object.hasOwnProperty.call(message, "maxResults"))
                writer.uint32(/* id 1, wireType 0 =*/8).int32(message.maxResults);
            return writer;
        };

        /**
         * Decodes a GetRecentPagesRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetRecentPagesRequest} GetRecentPagesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetRecentPagesRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetRecentPagesRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.maxResults = reader.int32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetRecentPagesRequest
         * @function getTypeUrl
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetRecentPagesRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetRecentPagesRequest";
        };

        return GetRecentPagesRequest;
    })();

    petajournal.GetRecentPagesResponse = (function() {

        /**
         * Properties of a GetRecentPagesResponse.
         * @memberof petajournal
         * @interface IGetRecentPagesResponse
         * @property {Array.<petajournal.IPage>|null} [pages] GetRecentPagesResponse pages
         */

        /**
         * Constructs a new GetRecentPagesResponse.
         * @memberof petajournal
         * @classdesc Represents a GetRecentPagesResponse.
         * @implements IGetRecentPagesResponse
         * @constructor
         * @param {petajournal.IGetRecentPagesResponse=} [properties] Properties to set
         */
        function GetRecentPagesResponse(properties) {
            this.pages = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetRecentPagesResponse pages.
         * @member {Array.<petajournal.IPage>} pages
         * @memberof petajournal.GetRecentPagesResponse
         * @instance
         */
        GetRecentPagesResponse.prototype.pages = $util.emptyArray;

        /**
         * Creates a new GetRecentPagesResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {petajournal.IGetRecentPagesResponse=} [properties] Properties to set
         * @returns {petajournal.GetRecentPagesResponse} GetRecentPagesResponse instance
         */
        GetRecentPagesResponse.create = function create(properties) {
            return new GetRecentPagesResponse(properties);
        };

        /**
         * Encodes the specified GetRecentPagesResponse message. Does not implicitly {@link petajournal.GetRecentPagesResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {petajournal.IGetRecentPagesResponse} message GetRecentPagesResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetRecentPagesResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pages != null && message.pages.length)
                for (let i = 0; i < message.pages.length; ++i)
                    $root.petajournal.Page.encode(message.pages[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a GetRecentPagesResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetRecentPagesResponse} GetRecentPagesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetRecentPagesResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetRecentPagesResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.pages && message.pages.length))
                            message.pages = [];
                        message.pages.push($root.petajournal.Page.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetRecentPagesResponse
         * @function getTypeUrl
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetRecentPagesResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetRecentPagesResponse";
        };

        return GetRecentPagesResponse;
    })();

    petajournal.GetBacklinksRequest = (function() {

        /**
         * Properties of a GetBacklinksRequest.
         * @memberof petajournal
         * @interface IGetBacklinksRequest
         * @property {string|null} [pageId] GetBacklinksRequest pageId
         */

        /**
         * Constructs a new GetBacklinksRequest.
         * @memberof petajournal
         * @classdesc Represents a GetBacklinksRequest.
         * @implements IGetBacklinksRequest
         * @constructor
         * @param {petajournal.IGetBacklinksRequest=} [properties] Properties to set
         */
        function GetBacklinksRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetBacklinksRequest pageId.
         * @member {string} pageId
         * @memberof petajournal.GetBacklinksRequest
         * @instance
         */
        GetBacklinksRequest.prototype.pageId = "";

        /**
         * Creates a new GetBacklinksRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {petajournal.IGetBacklinksRequest=} [properties] Properties to set
         * @returns {petajournal.GetBacklinksRequest} GetBacklinksRequest instance
         */
        GetBacklinksRequest.create = function create(properties) {
            return new GetBacklinksRequest(properties);
        };

        /**
         * Encodes the specified GetBacklinksRequest message. Does not implicitly {@link petajournal.GetBacklinksRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {petajournal.IGetBacklinksRequest} message GetBacklinksRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBacklinksRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            return writer;
        };

        /**
         * Decodes a GetBacklinksRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetBacklinksRequest} GetBacklinksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBacklinksRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetBacklinksRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetBacklinksRequest
         * @function getTypeUrl
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetBacklinksRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetBacklinksRequest";
        };

        return GetBacklinksRequest;
    })();

    petajournal.GetBacklinksResponse = (function() {

        /**
         * Properties of a GetBacklinksResponse.
         * @memberof petajournal
         * @interface IGetBacklinksResponse
         * @property {Array.<petajournal.IBacklinkEntry>|null} [entries] GetBacklinksResponse entries
         */

        /**
         * Constructs a new GetBacklinksResponse.
         * @memberof petajournal
         * @classdesc Represents a GetBacklinksResponse.
         * @implements IGetBacklinksResponse
         * @constructor
         * @param {petajournal.IGetBacklinksResponse=} [properties] Properties to set
         */
        function GetBacklinksResponse(properties) {
            this.entries = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetBacklinksResponse entries.
         * @member {Array.<petajournal.IBacklinkEntry>} entries
         * @memberof petajournal.GetBacklinksResponse
         * @instance
         */
        GetBacklinksResponse.prototype.entries = $util.emptyArray;

        /**
         * Creates a new GetBacklinksResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {petajournal.IGetBacklinksResponse=} [properties] Properties to set
         * @returns {petajournal.GetBacklinksResponse} GetBacklinksResponse instance
         */
        GetBacklinksResponse.create = function create(properties) {
            return new GetBacklinksResponse(properties);
        };

        /**
         * Encodes the specified GetBacklinksResponse message. Does not implicitly {@link petajournal.GetBacklinksResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {petajournal.IGetBacklinksResponse} message GetBacklinksResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBacklinksResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.entries != null && message.entries.length)
                for (let i = 0; i < message.entries.length; ++i)
                    $root.petajournal.BacklinkEntry.encode(message.entries[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a GetBacklinksResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetBacklinksResponse} GetBacklinksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBacklinksResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetBacklinksResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.entries && message.entries.length))
                            message.entries = [];
                        message.entries.push($root.petajournal.BacklinkEntry.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetBacklinksResponse
         * @function getTypeUrl
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetBacklinksResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetBacklinksResponse";
        };

        return GetBacklinksResponse;
    })();

    petajournal.BacklinkEntry = (function() {

        /**
         * Properties of a BacklinkEntry.
         * @memberof petajournal
         * @interface IBacklinkEntry
         * @property {petajournal.IPage|null} [sourcePage] BacklinkEntry sourcePage
         * @property {petajournal.IBlock|null} [sourceBlock] BacklinkEntry sourceBlock
         */

        /**
         * Constructs a new BacklinkEntry.
         * @memberof petajournal
         * @classdesc Represents a BacklinkEntry.
         * @implements IBacklinkEntry
         * @constructor
         * @param {petajournal.IBacklinkEntry=} [properties] Properties to set
         */
        function BacklinkEntry(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * BacklinkEntry sourcePage.
         * @member {petajournal.IPage|null|undefined} sourcePage
         * @memberof petajournal.BacklinkEntry
         * @instance
         */
        BacklinkEntry.prototype.sourcePage = null;

        /**
         * BacklinkEntry sourceBlock.
         * @member {petajournal.IBlock|null|undefined} sourceBlock
         * @memberof petajournal.BacklinkEntry
         * @instance
         */
        BacklinkEntry.prototype.sourceBlock = null;

        /**
         * Creates a new BacklinkEntry instance using the specified properties.
         * @function create
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {petajournal.IBacklinkEntry=} [properties] Properties to set
         * @returns {petajournal.BacklinkEntry} BacklinkEntry instance
         */
        BacklinkEntry.create = function create(properties) {
            return new BacklinkEntry(properties);
        };

        /**
         * Encodes the specified BacklinkEntry message. Does not implicitly {@link petajournal.BacklinkEntry.verify|verify} messages.
         * @function encode
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {petajournal.IBacklinkEntry} message BacklinkEntry message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        BacklinkEntry.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.sourcePage != null && Object.hasOwnProperty.call(message, "sourcePage"))
                $root.petajournal.Page.encode(message.sourcePage, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.sourceBlock != null && Object.hasOwnProperty.call(message, "sourceBlock"))
                $root.petajournal.Block.encode(message.sourceBlock, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a BacklinkEntry message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.BacklinkEntry} BacklinkEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        BacklinkEntry.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.BacklinkEntry();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.sourcePage = $root.petajournal.Page.decode(reader, reader.uint32());
                        break;
                    }
                case 2: {
                        message.sourceBlock = $root.petajournal.Block.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for BacklinkEntry
         * @function getTypeUrl
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        BacklinkEntry.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.BacklinkEntry";
        };

        return BacklinkEntry;
    })();

    petajournal.GetMostRecentJournalBeforeRequest = (function() {

        /**
         * Properties of a GetMostRecentJournalBeforeRequest.
         * @memberof petajournal
         * @interface IGetMostRecentJournalBeforeRequest
         * @property {string|null} [beforeDate] GetMostRecentJournalBeforeRequest beforeDate
         */

        /**
         * Constructs a new GetMostRecentJournalBeforeRequest.
         * @memberof petajournal
         * @classdesc Represents a GetMostRecentJournalBeforeRequest.
         * @implements IGetMostRecentJournalBeforeRequest
         * @constructor
         * @param {petajournal.IGetMostRecentJournalBeforeRequest=} [properties] Properties to set
         */
        function GetMostRecentJournalBeforeRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetMostRecentJournalBeforeRequest beforeDate.
         * @member {string} beforeDate
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @instance
         */
        GetMostRecentJournalBeforeRequest.prototype.beforeDate = "";

        /**
         * Creates a new GetMostRecentJournalBeforeRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {petajournal.IGetMostRecentJournalBeforeRequest=} [properties] Properties to set
         * @returns {petajournal.GetMostRecentJournalBeforeRequest} GetMostRecentJournalBeforeRequest instance
         */
        GetMostRecentJournalBeforeRequest.create = function create(properties) {
            return new GetMostRecentJournalBeforeRequest(properties);
        };

        /**
         * Encodes the specified GetMostRecentJournalBeforeRequest message. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {petajournal.IGetMostRecentJournalBeforeRequest} message GetMostRecentJournalBeforeRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetMostRecentJournalBeforeRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.beforeDate != null && Object.hasOwnProperty.call(message, "beforeDate"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.beforeDate);
            return writer;
        };

        /**
         * Decodes a GetMostRecentJournalBeforeRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetMostRecentJournalBeforeRequest} GetMostRecentJournalBeforeRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetMostRecentJournalBeforeRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetMostRecentJournalBeforeRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.beforeDate = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetMostRecentJournalBeforeRequest
         * @function getTypeUrl
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetMostRecentJournalBeforeRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetMostRecentJournalBeforeRequest";
        };

        return GetMostRecentJournalBeforeRequest;
    })();

    petajournal.GetMostRecentJournalBeforeResponse = (function() {

        /**
         * Properties of a GetMostRecentJournalBeforeResponse.
         * @memberof petajournal
         * @interface IGetMostRecentJournalBeforeResponse
         * @property {boolean|null} [found] GetMostRecentJournalBeforeResponse found
         * @property {petajournal.IPage|null} [page] GetMostRecentJournalBeforeResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetMostRecentJournalBeforeResponse blocks
         */

        /**
         * Constructs a new GetMostRecentJournalBeforeResponse.
         * @memberof petajournal
         * @classdesc Represents a GetMostRecentJournalBeforeResponse.
         * @implements IGetMostRecentJournalBeforeResponse
         * @constructor
         * @param {petajournal.IGetMostRecentJournalBeforeResponse=} [properties] Properties to set
         */
        function GetMostRecentJournalBeforeResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetMostRecentJournalBeforeResponse found.
         * @member {boolean} found
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @instance
         */
        GetMostRecentJournalBeforeResponse.prototype.found = false;

        /**
         * GetMostRecentJournalBeforeResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @instance
         */
        GetMostRecentJournalBeforeResponse.prototype.page = null;

        /**
         * GetMostRecentJournalBeforeResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @instance
         */
        GetMostRecentJournalBeforeResponse.prototype.blocks = $util.emptyArray;

        /**
         * Creates a new GetMostRecentJournalBeforeResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {petajournal.IGetMostRecentJournalBeforeResponse=} [properties] Properties to set
         * @returns {petajournal.GetMostRecentJournalBeforeResponse} GetMostRecentJournalBeforeResponse instance
         */
        GetMostRecentJournalBeforeResponse.create = function create(properties) {
            return new GetMostRecentJournalBeforeResponse(properties);
        };

        /**
         * Encodes the specified GetMostRecentJournalBeforeResponse message. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {petajournal.IGetMostRecentJournalBeforeResponse} message GetMostRecentJournalBeforeResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetMostRecentJournalBeforeResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.found != null && Object.hasOwnProperty.call(message, "found"))
                writer.uint32(/* id 1, wireType 0 =*/8).bool(message.found);
            if (message.page != null && Object.hasOwnProperty.call(message, "page"))
                $root.petajournal.Page.encode(message.page, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
            if (message.blocks != null && message.blocks.length)
                for (let i = 0; i < message.blocks.length; ++i)
                    $root.petajournal.Block.encode(message.blocks[i], writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a GetMostRecentJournalBeforeResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetMostRecentJournalBeforeResponse} GetMostRecentJournalBeforeResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetMostRecentJournalBeforeResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetMostRecentJournalBeforeResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.found = reader.bool();
                        break;
                    }
                case 2: {
                        message.page = $root.petajournal.Page.decode(reader, reader.uint32());
                        break;
                    }
                case 3: {
                        if (!(message.blocks && message.blocks.length))
                            message.blocks = [];
                        message.blocks.push($root.petajournal.Block.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetMostRecentJournalBeforeResponse
         * @function getTypeUrl
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetMostRecentJournalBeforeResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetMostRecentJournalBeforeResponse";
        };

        return GetMostRecentJournalBeforeResponse;
    })();

    petajournal.GetBlocksByStatusRequest = (function() {

        /**
         * Properties of a GetBlocksByStatusRequest.
         * @memberof petajournal
         * @interface IGetBlocksByStatusRequest
         * @property {Array.<string>|null} [statuses] GetBlocksByStatusRequest statuses
         */

        /**
         * Constructs a new GetBlocksByStatusRequest.
         * @memberof petajournal
         * @classdesc Represents a GetBlocksByStatusRequest.
         * @implements IGetBlocksByStatusRequest
         * @constructor
         * @param {petajournal.IGetBlocksByStatusRequest=} [properties] Properties to set
         */
        function GetBlocksByStatusRequest(properties) {
            this.statuses = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetBlocksByStatusRequest statuses.
         * @member {Array.<string>} statuses
         * @memberof petajournal.GetBlocksByStatusRequest
         * @instance
         */
        GetBlocksByStatusRequest.prototype.statuses = $util.emptyArray;

        /**
         * Creates a new GetBlocksByStatusRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {petajournal.IGetBlocksByStatusRequest=} [properties] Properties to set
         * @returns {petajournal.GetBlocksByStatusRequest} GetBlocksByStatusRequest instance
         */
        GetBlocksByStatusRequest.create = function create(properties) {
            return new GetBlocksByStatusRequest(properties);
        };

        /**
         * Encodes the specified GetBlocksByStatusRequest message. Does not implicitly {@link petajournal.GetBlocksByStatusRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {petajournal.IGetBlocksByStatusRequest} message GetBlocksByStatusRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBlocksByStatusRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.statuses != null && message.statuses.length)
                for (let i = 0; i < message.statuses.length; ++i)
                    writer.uint32(/* id 1, wireType 2 =*/10).string(message.statuses[i]);
            return writer;
        };

        /**
         * Decodes a GetBlocksByStatusRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetBlocksByStatusRequest} GetBlocksByStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBlocksByStatusRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetBlocksByStatusRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.statuses && message.statuses.length))
                            message.statuses = [];
                        message.statuses.push(reader.string());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetBlocksByStatusRequest
         * @function getTypeUrl
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetBlocksByStatusRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetBlocksByStatusRequest";
        };

        return GetBlocksByStatusRequest;
    })();

    petajournal.GetBlocksByStatusResponse = (function() {

        /**
         * Properties of a GetBlocksByStatusResponse.
         * @memberof petajournal
         * @interface IGetBlocksByStatusResponse
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetBlocksByStatusResponse blocks
         */

        /**
         * Constructs a new GetBlocksByStatusResponse.
         * @memberof petajournal
         * @classdesc Represents a GetBlocksByStatusResponse.
         * @implements IGetBlocksByStatusResponse
         * @constructor
         * @param {petajournal.IGetBlocksByStatusResponse=} [properties] Properties to set
         */
        function GetBlocksByStatusResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetBlocksByStatusResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetBlocksByStatusResponse
         * @instance
         */
        GetBlocksByStatusResponse.prototype.blocks = $util.emptyArray;

        /**
         * Creates a new GetBlocksByStatusResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {petajournal.IGetBlocksByStatusResponse=} [properties] Properties to set
         * @returns {petajournal.GetBlocksByStatusResponse} GetBlocksByStatusResponse instance
         */
        GetBlocksByStatusResponse.create = function create(properties) {
            return new GetBlocksByStatusResponse(properties);
        };

        /**
         * Encodes the specified GetBlocksByStatusResponse message. Does not implicitly {@link petajournal.GetBlocksByStatusResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {petajournal.IGetBlocksByStatusResponse} message GetBlocksByStatusResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBlocksByStatusResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blocks != null && message.blocks.length)
                for (let i = 0; i < message.blocks.length; ++i)
                    $root.petajournal.Block.encode(message.blocks[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a GetBlocksByStatusResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetBlocksByStatusResponse} GetBlocksByStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBlocksByStatusResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetBlocksByStatusResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.blocks && message.blocks.length))
                            message.blocks = [];
                        message.blocks.push($root.petajournal.Block.decode(reader, reader.uint32()));
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetBlocksByStatusResponse
         * @function getTypeUrl
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetBlocksByStatusResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetBlocksByStatusResponse";
        };

        return GetBlocksByStatusResponse;
    })();

    petajournal.GetJournalByNameRequest = (function() {

        /**
         * Properties of a GetJournalByNameRequest.
         * @memberof petajournal
         * @interface IGetJournalByNameRequest
         * @property {string|null} [fileName] GetJournalByNameRequest fileName
         */

        /**
         * Constructs a new GetJournalByNameRequest.
         * @memberof petajournal
         * @classdesc Represents a GetJournalByNameRequest.
         * @implements IGetJournalByNameRequest
         * @constructor
         * @param {petajournal.IGetJournalByNameRequest=} [properties] Properties to set
         */
        function GetJournalByNameRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetJournalByNameRequest fileName.
         * @member {string} fileName
         * @memberof petajournal.GetJournalByNameRequest
         * @instance
         */
        GetJournalByNameRequest.prototype.fileName = "";

        /**
         * Creates a new GetJournalByNameRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetJournalByNameRequest
         * @static
         * @param {petajournal.IGetJournalByNameRequest=} [properties] Properties to set
         * @returns {petajournal.GetJournalByNameRequest} GetJournalByNameRequest instance
         */
        GetJournalByNameRequest.create = function create(properties) {
            return new GetJournalByNameRequest(properties);
        };

        /**
         * Encodes the specified GetJournalByNameRequest message. Does not implicitly {@link petajournal.GetJournalByNameRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetJournalByNameRequest
         * @static
         * @param {petajournal.IGetJournalByNameRequest} message GetJournalByNameRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetJournalByNameRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.fileName != null && Object.hasOwnProperty.call(message, "fileName"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.fileName);
            return writer;
        };

        /**
         * Decodes a GetJournalByNameRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetJournalByNameRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetJournalByNameRequest} GetJournalByNameRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetJournalByNameRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetJournalByNameRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.fileName = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetJournalByNameRequest
         * @function getTypeUrl
         * @memberof petajournal.GetJournalByNameRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetJournalByNameRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetJournalByNameRequest";
        };

        return GetJournalByNameRequest;
    })();

    petajournal.GetJournalByNameResponse = (function() {

        /**
         * Properties of a GetJournalByNameResponse.
         * @memberof petajournal
         * @interface IGetJournalByNameResponse
         * @property {petajournal.IPage|null} [page] GetJournalByNameResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetJournalByNameResponse blocks
         * @property {boolean|null} [created] GetJournalByNameResponse created
         */

        /**
         * Constructs a new GetJournalByNameResponse.
         * @memberof petajournal
         * @classdesc Represents a GetJournalByNameResponse.
         * @implements IGetJournalByNameResponse
         * @constructor
         * @param {petajournal.IGetJournalByNameResponse=} [properties] Properties to set
         */
        function GetJournalByNameResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetJournalByNameResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetJournalByNameResponse
         * @instance
         */
        GetJournalByNameResponse.prototype.page = null;

        /**
         * GetJournalByNameResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetJournalByNameResponse
         * @instance
         */
        GetJournalByNameResponse.prototype.blocks = $util.emptyArray;

        /**
         * GetJournalByNameResponse created.
         * @member {boolean} created
         * @memberof petajournal.GetJournalByNameResponse
         * @instance
         */
        GetJournalByNameResponse.prototype.created = false;

        /**
         * Creates a new GetJournalByNameResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetJournalByNameResponse
         * @static
         * @param {petajournal.IGetJournalByNameResponse=} [properties] Properties to set
         * @returns {petajournal.GetJournalByNameResponse} GetJournalByNameResponse instance
         */
        GetJournalByNameResponse.create = function create(properties) {
            return new GetJournalByNameResponse(properties);
        };

        /**
         * Encodes the specified GetJournalByNameResponse message. Does not implicitly {@link petajournal.GetJournalByNameResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetJournalByNameResponse
         * @static
         * @param {petajournal.IGetJournalByNameResponse} message GetJournalByNameResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetJournalByNameResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.page != null && Object.hasOwnProperty.call(message, "page"))
                $root.petajournal.Page.encode(message.page, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.blocks != null && message.blocks.length)
                for (let i = 0; i < message.blocks.length; ++i)
                    $root.petajournal.Block.encode(message.blocks[i], writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
            if (message.created != null && Object.hasOwnProperty.call(message, "created"))
                writer.uint32(/* id 3, wireType 0 =*/24).bool(message.created);
            return writer;
        };

        /**
         * Decodes a GetJournalByNameResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetJournalByNameResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetJournalByNameResponse} GetJournalByNameResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetJournalByNameResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetJournalByNameResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.page = $root.petajournal.Page.decode(reader, reader.uint32());
                        break;
                    }
                case 2: {
                        if (!(message.blocks && message.blocks.length))
                            message.blocks = [];
                        message.blocks.push($root.petajournal.Block.decode(reader, reader.uint32()));
                        break;
                    }
                case 3: {
                        message.created = reader.bool();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetJournalByNameResponse
         * @function getTypeUrl
         * @memberof petajournal.GetJournalByNameResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetJournalByNameResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetJournalByNameResponse";
        };

        return GetJournalByNameResponse;
    })();

    petajournal.GetConfigRequest = (function() {

        /**
         * Properties of a GetConfigRequest.
         * @memberof petajournal
         * @interface IGetConfigRequest
         */

        /**
         * Constructs a new GetConfigRequest.
         * @memberof petajournal
         * @classdesc Represents a GetConfigRequest.
         * @implements IGetConfigRequest
         * @constructor
         * @param {petajournal.IGetConfigRequest=} [properties] Properties to set
         */
        function GetConfigRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Creates a new GetConfigRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {petajournal.IGetConfigRequest=} [properties] Properties to set
         * @returns {petajournal.GetConfigRequest} GetConfigRequest instance
         */
        GetConfigRequest.create = function create(properties) {
            return new GetConfigRequest(properties);
        };

        /**
         * Encodes the specified GetConfigRequest message. Does not implicitly {@link petajournal.GetConfigRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {petajournal.IGetConfigRequest} message GetConfigRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetConfigRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            return writer;
        };

        /**
         * Decodes a GetConfigRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetConfigRequest} GetConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetConfigRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetConfigRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetConfigRequest
         * @function getTypeUrl
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetConfigRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetConfigRequest";
        };

        return GetConfigRequest;
    })();

    petajournal.GetConfigResponse = (function() {

        /**
         * Properties of a GetConfigResponse.
         * @memberof petajournal
         * @interface IGetConfigResponse
         * @property {string|null} [weekNumbering] GetConfigResponse weekNumbering
         */

        /**
         * Constructs a new GetConfigResponse.
         * @memberof petajournal
         * @classdesc Represents a GetConfigResponse.
         * @implements IGetConfigResponse
         * @constructor
         * @param {petajournal.IGetConfigResponse=} [properties] Properties to set
         */
        function GetConfigResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetConfigResponse weekNumbering.
         * @member {string} weekNumbering
         * @memberof petajournal.GetConfigResponse
         * @instance
         */
        GetConfigResponse.prototype.weekNumbering = "";

        /**
         * Creates a new GetConfigResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {petajournal.IGetConfigResponse=} [properties] Properties to set
         * @returns {petajournal.GetConfigResponse} GetConfigResponse instance
         */
        GetConfigResponse.create = function create(properties) {
            return new GetConfigResponse(properties);
        };

        /**
         * Encodes the specified GetConfigResponse message. Does not implicitly {@link petajournal.GetConfigResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {petajournal.IGetConfigResponse} message GetConfigResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetConfigResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.weekNumbering != null && Object.hasOwnProperty.call(message, "weekNumbering"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.weekNumbering);
            return writer;
        };

        /**
         * Decodes a GetConfigResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetConfigResponse} GetConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetConfigResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetConfigResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.weekNumbering = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for GetConfigResponse
         * @function getTypeUrl
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetConfigResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetConfigResponse";
        };

        return GetConfigResponse;
    })();

    petajournal.UpdateConfigRequest = (function() {

        /**
         * Properties of an UpdateConfigRequest.
         * @memberof petajournal
         * @interface IUpdateConfigRequest
         * @property {string|null} [weekNumbering] UpdateConfigRequest weekNumbering
         */

        /**
         * Constructs a new UpdateConfigRequest.
         * @memberof petajournal
         * @classdesc Represents an UpdateConfigRequest.
         * @implements IUpdateConfigRequest
         * @constructor
         * @param {petajournal.IUpdateConfigRequest=} [properties] Properties to set
         */
        function UpdateConfigRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateConfigRequest weekNumbering.
         * @member {string} weekNumbering
         * @memberof petajournal.UpdateConfigRequest
         * @instance
         */
        UpdateConfigRequest.prototype.weekNumbering = "";

        /**
         * Creates a new UpdateConfigRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {petajournal.IUpdateConfigRequest=} [properties] Properties to set
         * @returns {petajournal.UpdateConfigRequest} UpdateConfigRequest instance
         */
        UpdateConfigRequest.create = function create(properties) {
            return new UpdateConfigRequest(properties);
        };

        /**
         * Encodes the specified UpdateConfigRequest message. Does not implicitly {@link petajournal.UpdateConfigRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {petajournal.IUpdateConfigRequest} message UpdateConfigRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateConfigRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.weekNumbering != null && Object.hasOwnProperty.call(message, "weekNumbering"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.weekNumbering);
            return writer;
        };

        /**
         * Decodes an UpdateConfigRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateConfigRequest} UpdateConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateConfigRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateConfigRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.weekNumbering = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdateConfigRequest
         * @function getTypeUrl
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateConfigRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateConfigRequest";
        };

        return UpdateConfigRequest;
    })();

    petajournal.UpdateConfigResponse = (function() {

        /**
         * Properties of an UpdateConfigResponse.
         * @memberof petajournal
         * @interface IUpdateConfigResponse
         * @property {string|null} [weekNumbering] UpdateConfigResponse weekNumbering
         */

        /**
         * Constructs a new UpdateConfigResponse.
         * @memberof petajournal
         * @classdesc Represents an UpdateConfigResponse.
         * @implements IUpdateConfigResponse
         * @constructor
         * @param {petajournal.IUpdateConfigResponse=} [properties] Properties to set
         */
        function UpdateConfigResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateConfigResponse weekNumbering.
         * @member {string} weekNumbering
         * @memberof petajournal.UpdateConfigResponse
         * @instance
         */
        UpdateConfigResponse.prototype.weekNumbering = "";

        /**
         * Creates a new UpdateConfigResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {petajournal.IUpdateConfigResponse=} [properties] Properties to set
         * @returns {petajournal.UpdateConfigResponse} UpdateConfigResponse instance
         */
        UpdateConfigResponse.create = function create(properties) {
            return new UpdateConfigResponse(properties);
        };

        /**
         * Encodes the specified UpdateConfigResponse message. Does not implicitly {@link petajournal.UpdateConfigResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {petajournal.IUpdateConfigResponse} message UpdateConfigResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateConfigResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.weekNumbering != null && Object.hasOwnProperty.call(message, "weekNumbering"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.weekNumbering);
            return writer;
        };

        /**
         * Decodes an UpdateConfigResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateConfigResponse} UpdateConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateConfigResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateConfigResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.weekNumbering = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for UpdateConfigResponse
         * @function getTypeUrl
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateConfigResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateConfigResponse";
        };

        return UpdateConfigResponse;
    })();

    petajournal.ExportTreeMarkdownRequest = (function() {

        /**
         * Properties of an ExportTreeMarkdownRequest.
         * @memberof petajournal
         * @interface IExportTreeMarkdownRequest
         * @property {Array.<string>|null} [blockIds] ExportTreeMarkdownRequest blockIds
         */

        /**
         * Constructs a new ExportTreeMarkdownRequest.
         * @memberof petajournal
         * @classdesc Represents an ExportTreeMarkdownRequest.
         * @implements IExportTreeMarkdownRequest
         * @constructor
         * @param {petajournal.IExportTreeMarkdownRequest=} [properties] Properties to set
         */
        function ExportTreeMarkdownRequest(properties) {
            this.blockIds = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ExportTreeMarkdownRequest blockIds.
         * @member {Array.<string>} blockIds
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @instance
         */
        ExportTreeMarkdownRequest.prototype.blockIds = $util.emptyArray;

        /**
         * Creates a new ExportTreeMarkdownRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {petajournal.IExportTreeMarkdownRequest=} [properties] Properties to set
         * @returns {petajournal.ExportTreeMarkdownRequest} ExportTreeMarkdownRequest instance
         */
        ExportTreeMarkdownRequest.create = function create(properties) {
            return new ExportTreeMarkdownRequest(properties);
        };

        /**
         * Encodes the specified ExportTreeMarkdownRequest message. Does not implicitly {@link petajournal.ExportTreeMarkdownRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {petajournal.IExportTreeMarkdownRequest} message ExportTreeMarkdownRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ExportTreeMarkdownRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.blockIds != null && message.blockIds.length)
                for (let i = 0; i < message.blockIds.length; ++i)
                    writer.uint32(/* id 1, wireType 2 =*/10).string(message.blockIds[i]);
            return writer;
        };

        /**
         * Decodes an ExportTreeMarkdownRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ExportTreeMarkdownRequest} ExportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ExportTreeMarkdownRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ExportTreeMarkdownRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.blockIds && message.blockIds.length))
                            message.blockIds = [];
                        message.blockIds.push(reader.string());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ExportTreeMarkdownRequest
         * @function getTypeUrl
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ExportTreeMarkdownRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ExportTreeMarkdownRequest";
        };

        return ExportTreeMarkdownRequest;
    })();

    petajournal.ExportTreeMarkdownResponse = (function() {

        /**
         * Properties of an ExportTreeMarkdownResponse.
         * @memberof petajournal
         * @interface IExportTreeMarkdownResponse
         * @property {string|null} [treeMarkdown] ExportTreeMarkdownResponse treeMarkdown
         * @property {string|null} [html] ExportTreeMarkdownResponse html
         * @property {string|null} [flatMarkdown] ExportTreeMarkdownResponse flatMarkdown
         */

        /**
         * Constructs a new ExportTreeMarkdownResponse.
         * @memberof petajournal
         * @classdesc Represents an ExportTreeMarkdownResponse.
         * @implements IExportTreeMarkdownResponse
         * @constructor
         * @param {petajournal.IExportTreeMarkdownResponse=} [properties] Properties to set
         */
        function ExportTreeMarkdownResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ExportTreeMarkdownResponse treeMarkdown.
         * @member {string} treeMarkdown
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @instance
         */
        ExportTreeMarkdownResponse.prototype.treeMarkdown = "";

        /**
         * ExportTreeMarkdownResponse html.
         * @member {string} html
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @instance
         */
        ExportTreeMarkdownResponse.prototype.html = "";

        /**
         * ExportTreeMarkdownResponse flatMarkdown.
         * @member {string} flatMarkdown
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @instance
         */
        ExportTreeMarkdownResponse.prototype.flatMarkdown = "";

        /**
         * Creates a new ExportTreeMarkdownResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {petajournal.IExportTreeMarkdownResponse=} [properties] Properties to set
         * @returns {petajournal.ExportTreeMarkdownResponse} ExportTreeMarkdownResponse instance
         */
        ExportTreeMarkdownResponse.create = function create(properties) {
            return new ExportTreeMarkdownResponse(properties);
        };

        /**
         * Encodes the specified ExportTreeMarkdownResponse message. Does not implicitly {@link petajournal.ExportTreeMarkdownResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {petajournal.IExportTreeMarkdownResponse} message ExportTreeMarkdownResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ExportTreeMarkdownResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.treeMarkdown != null && Object.hasOwnProperty.call(message, "treeMarkdown"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.treeMarkdown);
            if (message.html != null && Object.hasOwnProperty.call(message, "html"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.html);
            if (message.flatMarkdown != null && Object.hasOwnProperty.call(message, "flatMarkdown"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.flatMarkdown);
            return writer;
        };

        /**
         * Decodes an ExportTreeMarkdownResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ExportTreeMarkdownResponse} ExportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ExportTreeMarkdownResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ExportTreeMarkdownResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.treeMarkdown = reader.string();
                        break;
                    }
                case 2: {
                        message.html = reader.string();
                        break;
                    }
                case 3: {
                        message.flatMarkdown = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ExportTreeMarkdownResponse
         * @function getTypeUrl
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ExportTreeMarkdownResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ExportTreeMarkdownResponse";
        };

        return ExportTreeMarkdownResponse;
    })();

    petajournal.ImportTreeMarkdownRequest = (function() {

        /**
         * Properties of an ImportTreeMarkdownRequest.
         * @memberof petajournal
         * @interface IImportTreeMarkdownRequest
         * @property {string|null} [treeMarkdown] ImportTreeMarkdownRequest treeMarkdown
         */

        /**
         * Constructs a new ImportTreeMarkdownRequest.
         * @memberof petajournal
         * @classdesc Represents an ImportTreeMarkdownRequest.
         * @implements IImportTreeMarkdownRequest
         * @constructor
         * @param {petajournal.IImportTreeMarkdownRequest=} [properties] Properties to set
         */
        function ImportTreeMarkdownRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ImportTreeMarkdownRequest treeMarkdown.
         * @member {string} treeMarkdown
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @instance
         */
        ImportTreeMarkdownRequest.prototype.treeMarkdown = "";

        /**
         * Creates a new ImportTreeMarkdownRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {petajournal.IImportTreeMarkdownRequest=} [properties] Properties to set
         * @returns {petajournal.ImportTreeMarkdownRequest} ImportTreeMarkdownRequest instance
         */
        ImportTreeMarkdownRequest.create = function create(properties) {
            return new ImportTreeMarkdownRequest(properties);
        };

        /**
         * Encodes the specified ImportTreeMarkdownRequest message. Does not implicitly {@link petajournal.ImportTreeMarkdownRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {petajournal.IImportTreeMarkdownRequest} message ImportTreeMarkdownRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ImportTreeMarkdownRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.treeMarkdown != null && Object.hasOwnProperty.call(message, "treeMarkdown"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.treeMarkdown);
            return writer;
        };

        /**
         * Decodes an ImportTreeMarkdownRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ImportTreeMarkdownRequest} ImportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ImportTreeMarkdownRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ImportTreeMarkdownRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.treeMarkdown = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ImportTreeMarkdownRequest
         * @function getTypeUrl
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ImportTreeMarkdownRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ImportTreeMarkdownRequest";
        };

        return ImportTreeMarkdownRequest;
    })();

    petajournal.ImportTreeMarkdownResponse = (function() {

        /**
         * Properties of an ImportTreeMarkdownResponse.
         * @memberof petajournal
         * @interface IImportTreeMarkdownResponse
         * @property {string|null} [flatMarkdown] ImportTreeMarkdownResponse flatMarkdown
         */

        /**
         * Constructs a new ImportTreeMarkdownResponse.
         * @memberof petajournal
         * @classdesc Represents an ImportTreeMarkdownResponse.
         * @implements IImportTreeMarkdownResponse
         * @constructor
         * @param {petajournal.IImportTreeMarkdownResponse=} [properties] Properties to set
         */
        function ImportTreeMarkdownResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ImportTreeMarkdownResponse flatMarkdown.
         * @member {string} flatMarkdown
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @instance
         */
        ImportTreeMarkdownResponse.prototype.flatMarkdown = "";

        /**
         * Creates a new ImportTreeMarkdownResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {petajournal.IImportTreeMarkdownResponse=} [properties] Properties to set
         * @returns {petajournal.ImportTreeMarkdownResponse} ImportTreeMarkdownResponse instance
         */
        ImportTreeMarkdownResponse.create = function create(properties) {
            return new ImportTreeMarkdownResponse(properties);
        };

        /**
         * Encodes the specified ImportTreeMarkdownResponse message. Does not implicitly {@link petajournal.ImportTreeMarkdownResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {petajournal.IImportTreeMarkdownResponse} message ImportTreeMarkdownResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ImportTreeMarkdownResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.flatMarkdown != null && Object.hasOwnProperty.call(message, "flatMarkdown"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.flatMarkdown);
            return writer;
        };

        /**
         * Decodes an ImportTreeMarkdownResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ImportTreeMarkdownResponse} ImportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ImportTreeMarkdownResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ImportTreeMarkdownResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.flatMarkdown = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ImportTreeMarkdownResponse
         * @function getTypeUrl
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ImportTreeMarkdownResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ImportTreeMarkdownResponse";
        };

        return ImportTreeMarkdownResponse;
    })();

    petajournal.ServerEvent = (function() {

        /**
         * Properties of a ServerEvent.
         * @memberof petajournal
         * @interface IServerEvent
         * @property {string|null} [originatorSessionId] ServerEvent originatorSessionId
         * @property {petajournal.IConnectedEvent|null} [connected] ServerEvent connected
         * @property {petajournal.IPagesUpdatedEvent|null} [pagesUpdated] ServerEvent pagesUpdated
         */

        /**
         * Constructs a new ServerEvent.
         * @memberof petajournal
         * @classdesc Represents a ServerEvent.
         * @implements IServerEvent
         * @constructor
         * @param {petajournal.IServerEvent=} [properties] Properties to set
         */
        function ServerEvent(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ServerEvent originatorSessionId.
         * @member {string} originatorSessionId
         * @memberof petajournal.ServerEvent
         * @instance
         */
        ServerEvent.prototype.originatorSessionId = "";

        /**
         * ServerEvent connected.
         * @member {petajournal.IConnectedEvent|null|undefined} connected
         * @memberof petajournal.ServerEvent
         * @instance
         */
        ServerEvent.prototype.connected = null;

        /**
         * ServerEvent pagesUpdated.
         * @member {petajournal.IPagesUpdatedEvent|null|undefined} pagesUpdated
         * @memberof petajournal.ServerEvent
         * @instance
         */
        ServerEvent.prototype.pagesUpdated = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        /**
         * ServerEvent event.
         * @member {"connected"|"pagesUpdated"|undefined} event
         * @memberof petajournal.ServerEvent
         * @instance
         */
        Object.defineProperty(ServerEvent.prototype, "event", {
            get: $util.oneOfGetter($oneOfFields = ["connected", "pagesUpdated"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new ServerEvent instance using the specified properties.
         * @function create
         * @memberof petajournal.ServerEvent
         * @static
         * @param {petajournal.IServerEvent=} [properties] Properties to set
         * @returns {petajournal.ServerEvent} ServerEvent instance
         */
        ServerEvent.create = function create(properties) {
            return new ServerEvent(properties);
        };

        /**
         * Encodes the specified ServerEvent message. Does not implicitly {@link petajournal.ServerEvent.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ServerEvent
         * @static
         * @param {petajournal.IServerEvent} message ServerEvent message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ServerEvent.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.originatorSessionId != null && Object.hasOwnProperty.call(message, "originatorSessionId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.originatorSessionId);
            if (message.connected != null && Object.hasOwnProperty.call(message, "connected"))
                $root.petajournal.ConnectedEvent.encode(message.connected, writer.uint32(/* id 10, wireType 2 =*/82).fork()).ldelim();
            if (message.pagesUpdated != null && Object.hasOwnProperty.call(message, "pagesUpdated"))
                $root.petajournal.PagesUpdatedEvent.encode(message.pagesUpdated, writer.uint32(/* id 20, wireType 2 =*/162).fork()).ldelim();
            return writer;
        };

        /**
         * Decodes a ServerEvent message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ServerEvent
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ServerEvent} ServerEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ServerEvent.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ServerEvent();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.originatorSessionId = reader.string();
                        break;
                    }
                case 10: {
                        message.connected = $root.petajournal.ConnectedEvent.decode(reader, reader.uint32());
                        break;
                    }
                case 20: {
                        message.pagesUpdated = $root.petajournal.PagesUpdatedEvent.decode(reader, reader.uint32());
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ServerEvent
         * @function getTypeUrl
         * @memberof petajournal.ServerEvent
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ServerEvent.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ServerEvent";
        };

        return ServerEvent;
    })();

    petajournal.ConnectedEvent = (function() {

        /**
         * Properties of a ConnectedEvent.
         * @memberof petajournal
         * @interface IConnectedEvent
         * @property {string|null} [sessionId] ConnectedEvent sessionId
         */

        /**
         * Constructs a new ConnectedEvent.
         * @memberof petajournal
         * @classdesc Represents a ConnectedEvent.
         * @implements IConnectedEvent
         * @constructor
         * @param {petajournal.IConnectedEvent=} [properties] Properties to set
         */
        function ConnectedEvent(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ConnectedEvent sessionId.
         * @member {string} sessionId
         * @memberof petajournal.ConnectedEvent
         * @instance
         */
        ConnectedEvent.prototype.sessionId = "";

        /**
         * Creates a new ConnectedEvent instance using the specified properties.
         * @function create
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {petajournal.IConnectedEvent=} [properties] Properties to set
         * @returns {petajournal.ConnectedEvent} ConnectedEvent instance
         */
        ConnectedEvent.create = function create(properties) {
            return new ConnectedEvent(properties);
        };

        /**
         * Encodes the specified ConnectedEvent message. Does not implicitly {@link petajournal.ConnectedEvent.verify|verify} messages.
         * @function encode
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {petajournal.IConnectedEvent} message ConnectedEvent message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ConnectedEvent.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.sessionId != null && Object.hasOwnProperty.call(message, "sessionId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.sessionId);
            return writer;
        };

        /**
         * Decodes a ConnectedEvent message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.ConnectedEvent} ConnectedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ConnectedEvent.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.ConnectedEvent();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.sessionId = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for ConnectedEvent
         * @function getTypeUrl
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ConnectedEvent.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.ConnectedEvent";
        };

        return ConnectedEvent;
    })();

    petajournal.PagesUpdatedEvent = (function() {

        /**
         * Properties of a PagesUpdatedEvent.
         * @memberof petajournal
         * @interface IPagesUpdatedEvent
         * @property {string|null} [pageId] PagesUpdatedEvent pageId
         */

        /**
         * Constructs a new PagesUpdatedEvent.
         * @memberof petajournal
         * @classdesc Represents a PagesUpdatedEvent.
         * @implements IPagesUpdatedEvent
         * @constructor
         * @param {petajournal.IPagesUpdatedEvent=} [properties] Properties to set
         */
        function PagesUpdatedEvent(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * PagesUpdatedEvent pageId.
         * @member {string} pageId
         * @memberof petajournal.PagesUpdatedEvent
         * @instance
         */
        PagesUpdatedEvent.prototype.pageId = "";

        /**
         * Creates a new PagesUpdatedEvent instance using the specified properties.
         * @function create
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {petajournal.IPagesUpdatedEvent=} [properties] Properties to set
         * @returns {petajournal.PagesUpdatedEvent} PagesUpdatedEvent instance
         */
        PagesUpdatedEvent.create = function create(properties) {
            return new PagesUpdatedEvent(properties);
        };

        /**
         * Encodes the specified PagesUpdatedEvent message. Does not implicitly {@link petajournal.PagesUpdatedEvent.verify|verify} messages.
         * @function encode
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {petajournal.IPagesUpdatedEvent} message PagesUpdatedEvent message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        PagesUpdatedEvent.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            return writer;
        };

        /**
         * Decodes a PagesUpdatedEvent message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.PagesUpdatedEvent} PagesUpdatedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        PagesUpdatedEvent.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.PagesUpdatedEvent();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.pageId = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Gets the default type url for PagesUpdatedEvent
         * @function getTypeUrl
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        PagesUpdatedEvent.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.PagesUpdatedEvent";
        };

        return PagesUpdatedEvent;
    })();

    return petajournal;
})();

export { $root as default };
