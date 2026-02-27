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
         * Encodes the specified Timestamp message, length delimited. Does not implicitly {@link petajournal.Timestamp.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.Timestamp
         * @static
         * @param {petajournal.ITimestamp} message Timestamp message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Timestamp.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a Timestamp message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.Timestamp
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.Timestamp} Timestamp
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Timestamp.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Timestamp message.
         * @function verify
         * @memberof petajournal.Timestamp
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Timestamp.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.millis != null && message.hasOwnProperty("millis"))
                if (!$util.isInteger(message.millis) && !(message.millis && $util.isInteger(message.millis.low) && $util.isInteger(message.millis.high)))
                    return "millis: integer|Long expected";
            return null;
        };

        /**
         * Creates a Timestamp message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.Timestamp
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.Timestamp} Timestamp
         */
        Timestamp.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.Timestamp)
                return object;
            let message = new $root.petajournal.Timestamp();
            if (object.millis != null)
                if ($util.Long)
                    (message.millis = $util.Long.fromValue(object.millis)).unsigned = false;
                else if (typeof object.millis === "string")
                    message.millis = parseInt(object.millis, 10);
                else if (typeof object.millis === "number")
                    message.millis = object.millis;
                else if (typeof object.millis === "object")
                    message.millis = new $util.LongBits(object.millis.low >>> 0, object.millis.high >>> 0).toNumber();
            return message;
        };

        /**
         * Creates a plain object from a Timestamp message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.Timestamp
         * @static
         * @param {petajournal.Timestamp} message Timestamp
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Timestamp.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                if ($util.Long) {
                    let long = new $util.Long(0, 0, false);
                    object.millis = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                } else
                    object.millis = options.longs === String ? "0" : 0;
            if (message.millis != null && message.hasOwnProperty("millis"))
                if (typeof message.millis === "number")
                    object.millis = options.longs === String ? String(message.millis) : message.millis;
                else
                    object.millis = options.longs === String ? $util.Long.prototype.toString.call(message.millis) : options.longs === Number ? new $util.LongBits(message.millis.low >>> 0, message.millis.high >>> 0).toNumber() : message.millis;
            return object;
        };

        /**
         * Converts this Timestamp to JSON.
         * @function toJSON
         * @memberof petajournal.Timestamp
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Timestamp.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * @property {string|null} [name] Page name
         * @property {string|null} [type] Page type
         * @property {petajournal.ITimestamp|null} [createdDate] Page createdDate
         * @property {petajournal.ITimestamp|null} [modifiedDate] Page modifiedDate
         * @property {Array.<string>|null} [tags] Page tags
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
         * Page name.
         * @member {string} name
         * @memberof petajournal.Page
         * @instance
         */
        Page.prototype.name = "";

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
            if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.name);
            if (message.createdDate != null && Object.hasOwnProperty.call(message, "createdDate"))
                $root.petajournal.Timestamp.encode(message.createdDate, writer.uint32(/* id 4, wireType 2 =*/34).fork()).ldelim();
            if (message.modifiedDate != null && Object.hasOwnProperty.call(message, "modifiedDate"))
                $root.petajournal.Timestamp.encode(message.modifiedDate, writer.uint32(/* id 5, wireType 2 =*/42).fork()).ldelim();
            if (message.tags != null && message.tags.length)
                for (let i = 0; i < message.tags.length; ++i)
                    writer.uint32(/* id 6, wireType 2 =*/50).string(message.tags[i]);
            if (message.type != null && Object.hasOwnProperty.call(message, "type"))
                writer.uint32(/* id 9, wireType 2 =*/74).string(message.type);
            return writer;
        };

        /**
         * Encodes the specified Page message, length delimited. Does not implicitly {@link petajournal.Page.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.Page
         * @static
         * @param {petajournal.IPage} message Page message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Page.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                        message.name = reader.string();
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
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Decodes a Page message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.Page
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.Page} Page
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Page.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Page message.
         * @function verify
         * @memberof petajournal.Page
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Page.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.id != null && message.hasOwnProperty("id"))
                if (!$util.isString(message.id))
                    return "id: string expected";
            if (message.dirPath != null && message.hasOwnProperty("dirPath"))
                if (!$util.isString(message.dirPath))
                    return "dirPath: string expected";
            if (message.name != null && message.hasOwnProperty("name"))
                if (!$util.isString(message.name))
                    return "name: string expected";
            if (message.type != null && message.hasOwnProperty("type"))
                if (!$util.isString(message.type))
                    return "type: string expected";
            if (message.createdDate != null && message.hasOwnProperty("createdDate")) {
                let error = $root.petajournal.Timestamp.verify(message.createdDate);
                if (error)
                    return "createdDate." + error;
            }
            if (message.modifiedDate != null && message.hasOwnProperty("modifiedDate")) {
                let error = $root.petajournal.Timestamp.verify(message.modifiedDate);
                if (error)
                    return "modifiedDate." + error;
            }
            if (message.tags != null && message.hasOwnProperty("tags")) {
                if (!Array.isArray(message.tags))
                    return "tags: array expected";
                for (let i = 0; i < message.tags.length; ++i)
                    if (!$util.isString(message.tags[i]))
                        return "tags: string[] expected";
            }
            return null;
        };

        /**
         * Creates a Page message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.Page
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.Page} Page
         */
        Page.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.Page)
                return object;
            let message = new $root.petajournal.Page();
            if (object.id != null)
                message.id = String(object.id);
            if (object.dirPath != null)
                message.dirPath = String(object.dirPath);
            if (object.name != null)
                message.name = String(object.name);
            if (object.type != null)
                message.type = String(object.type);
            if (object.createdDate != null) {
                if (typeof object.createdDate !== "object")
                    throw TypeError(".petajournal.Page.createdDate: object expected");
                message.createdDate = $root.petajournal.Timestamp.fromObject(object.createdDate);
            }
            if (object.modifiedDate != null) {
                if (typeof object.modifiedDate !== "object")
                    throw TypeError(".petajournal.Page.modifiedDate: object expected");
                message.modifiedDate = $root.petajournal.Timestamp.fromObject(object.modifiedDate);
            }
            if (object.tags) {
                if (!Array.isArray(object.tags))
                    throw TypeError(".petajournal.Page.tags: array expected");
                message.tags = [];
                for (let i = 0; i < object.tags.length; ++i)
                    message.tags[i] = String(object.tags[i]);
            }
            return message;
        };

        /**
         * Creates a plain object from a Page message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.Page
         * @static
         * @param {petajournal.Page} message Page
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Page.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.tags = [];
            if (options.defaults) {
                object.id = "";
                object.dirPath = "";
                object.name = "";
                object.createdDate = null;
                object.modifiedDate = null;
                object.type = "";
            }
            if (message.id != null && message.hasOwnProperty("id"))
                object.id = message.id;
            if (message.dirPath != null && message.hasOwnProperty("dirPath"))
                object.dirPath = message.dirPath;
            if (message.name != null && message.hasOwnProperty("name"))
                object.name = message.name;
            if (message.createdDate != null && message.hasOwnProperty("createdDate"))
                object.createdDate = $root.petajournal.Timestamp.toObject(message.createdDate, options);
            if (message.modifiedDate != null && message.hasOwnProperty("modifiedDate"))
                object.modifiedDate = $root.petajournal.Timestamp.toObject(message.modifiedDate, options);
            if (message.tags && message.tags.length) {
                object.tags = [];
                for (let j = 0; j < message.tags.length; ++j)
                    object.tags[j] = message.tags[j];
            }
            if (message.type != null && message.hasOwnProperty("type"))
                object.type = message.type;
            return object;
        };

        /**
         * Converts this Page to JSON.
         * @function toJSON
         * @memberof petajournal.Page
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Page.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * @property {string|null} [name] Block name
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
         * Block name.
         * @member {string} name
         * @memberof petajournal.Block
         * @instance
         */
        Block.prototype.name = "";

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
            if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.name);
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
            if (message.type != null && Object.hasOwnProperty.call(message, "type"))
                writer.uint32(/* id 15, wireType 2 =*/122).string(message.type);
            return writer;
        };

        /**
         * Encodes the specified Block message, length delimited. Does not implicitly {@link petajournal.Block.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.Block
         * @static
         * @param {petajournal.IBlock} message Block message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Block.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                        message.name = reader.string();
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
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Decodes a Block message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.Block
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.Block} Block
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Block.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Block message.
         * @function verify
         * @memberof petajournal.Block
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Block.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.id != null && message.hasOwnProperty("id"))
                if (!$util.isString(message.id))
                    return "id: string expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            if (message.dirPath != null && message.hasOwnProperty("dirPath"))
                if (!$util.isString(message.dirPath))
                    return "dirPath: string expected";
            if (message.name != null && message.hasOwnProperty("name"))
                if (!$util.isString(message.name))
                    return "name: string expected";
            if (message.type != null && message.hasOwnProperty("type"))
                if (!$util.isString(message.type))
                    return "type: string expected";
            if (message.createdDate != null && message.hasOwnProperty("createdDate")) {
                let error = $root.petajournal.Timestamp.verify(message.createdDate);
                if (error)
                    return "createdDate." + error;
            }
            if (message.modifiedDate != null && message.hasOwnProperty("modifiedDate")) {
                let error = $root.petajournal.Timestamp.verify(message.modifiedDate);
                if (error)
                    return "modifiedDate." + error;
            }
            if (message.idx != null && message.hasOwnProperty("idx"))
                if (!$util.isString(message.idx))
                    return "idx: string expected";
            if (message.level != null && message.hasOwnProperty("level"))
                if (!$util.isInteger(message.level))
                    return "level: integer expected";
            if (message.body != null && message.hasOwnProperty("body"))
                if (!$util.isString(message.body))
                    return "body: string expected";
            if (message.html != null && message.hasOwnProperty("html"))
                if (!$util.isString(message.html))
                    return "html: string expected";
            if (message.collapsed != null && message.hasOwnProperty("collapsed"))
                if (typeof message.collapsed !== "boolean")
                    return "collapsed: boolean expected";
            if (message.status != null && message.hasOwnProperty("status"))
                if (!$util.isString(message.status))
                    return "status: string expected";
            if (message.types != null && message.hasOwnProperty("types")) {
                if (!Array.isArray(message.types))
                    return "types: array expected";
                for (let i = 0; i < message.types.length; ++i)
                    if (!$util.isString(message.types[i]))
                        return "types: string[] expected";
            }
            return null;
        };

        /**
         * Creates a Block message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.Block
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.Block} Block
         */
        Block.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.Block)
                return object;
            let message = new $root.petajournal.Block();
            if (object.id != null)
                message.id = String(object.id);
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            if (object.dirPath != null)
                message.dirPath = String(object.dirPath);
            if (object.name != null)
                message.name = String(object.name);
            if (object.type != null)
                message.type = String(object.type);
            if (object.createdDate != null) {
                if (typeof object.createdDate !== "object")
                    throw TypeError(".petajournal.Block.createdDate: object expected");
                message.createdDate = $root.petajournal.Timestamp.fromObject(object.createdDate);
            }
            if (object.modifiedDate != null) {
                if (typeof object.modifiedDate !== "object")
                    throw TypeError(".petajournal.Block.modifiedDate: object expected");
                message.modifiedDate = $root.petajournal.Timestamp.fromObject(object.modifiedDate);
            }
            if (object.idx != null)
                message.idx = String(object.idx);
            if (object.level != null)
                message.level = object.level | 0;
            if (object.body != null)
                message.body = String(object.body);
            if (object.html != null)
                message.html = String(object.html);
            if (object.collapsed != null)
                message.collapsed = Boolean(object.collapsed);
            if (object.status != null)
                message.status = String(object.status);
            if (object.types) {
                if (!Array.isArray(object.types))
                    throw TypeError(".petajournal.Block.types: array expected");
                message.types = [];
                for (let i = 0; i < object.types.length; ++i)
                    message.types[i] = String(object.types[i]);
            }
            return message;
        };

        /**
         * Creates a plain object from a Block message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.Block
         * @static
         * @param {petajournal.Block} message Block
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Block.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.types = [];
            if (options.defaults) {
                object.id = "";
                object.pageId = "";
                object.dirPath = "";
                object.name = "";
                object.createdDate = null;
                object.modifiedDate = null;
                object.idx = "";
                object.level = 0;
                object.body = "";
                object.html = "";
                object.collapsed = false;
                object.status = "";
                object.type = "";
            }
            if (message.id != null && message.hasOwnProperty("id"))
                object.id = message.id;
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            if (message.dirPath != null && message.hasOwnProperty("dirPath"))
                object.dirPath = message.dirPath;
            if (message.name != null && message.hasOwnProperty("name"))
                object.name = message.name;
            if (message.createdDate != null && message.hasOwnProperty("createdDate"))
                object.createdDate = $root.petajournal.Timestamp.toObject(message.createdDate, options);
            if (message.modifiedDate != null && message.hasOwnProperty("modifiedDate"))
                object.modifiedDate = $root.petajournal.Timestamp.toObject(message.modifiedDate, options);
            if (message.idx != null && message.hasOwnProperty("idx"))
                object.idx = message.idx;
            if (message.level != null && message.hasOwnProperty("level"))
                object.level = message.level;
            if (message.body != null && message.hasOwnProperty("body"))
                object.body = message.body;
            if (message.html != null && message.hasOwnProperty("html"))
                object.html = message.html;
            if (message.collapsed != null && message.hasOwnProperty("collapsed"))
                object.collapsed = message.collapsed;
            if (message.status != null && message.hasOwnProperty("status"))
                object.status = message.status;
            if (message.types && message.types.length) {
                object.types = [];
                for (let j = 0; j < message.types.length; ++j)
                    object.types[j] = message.types[j];
            }
            if (message.type != null && message.hasOwnProperty("type"))
                object.type = message.type;
            return object;
        };

        /**
         * Converts this Block to JSON.
         * @function toJSON
         * @memberof petajournal.Block
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Block.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchRequest message, length delimited. Does not implicitly {@link petajournal.SearchRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchRequest
         * @static
         * @param {petajournal.ISearchRequest} message SearchRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchRequest} SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchRequest message.
         * @function verify
         * @memberof petajournal.SearchRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.query != null && message.hasOwnProperty("query"))
                if (!$util.isString(message.query))
                    return "query: string expected";
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                if (!$util.isInteger(message.maxResults))
                    return "maxResults: integer expected";
            return null;
        };

        /**
         * Creates a SearchRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchRequest} SearchRequest
         */
        SearchRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchRequest)
                return object;
            let message = new $root.petajournal.SearchRequest();
            if (object.query != null)
                message.query = String(object.query);
            if (object.maxResults != null)
                message.maxResults = object.maxResults | 0;
            return message;
        };

        /**
         * Creates a plain object from a SearchRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchRequest
         * @static
         * @param {petajournal.SearchRequest} message SearchRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.query = "";
                object.maxResults = 0;
            }
            if (message.query != null && message.hasOwnProperty("query"))
                object.query = message.query;
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                object.maxResults = message.maxResults;
            return object;
        };

        /**
         * Converts this SearchRequest to JSON.
         * @function toJSON
         * @memberof petajournal.SearchRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchResponse message, length delimited. Does not implicitly {@link petajournal.SearchResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchResponse
         * @static
         * @param {petajournal.ISearchResponse} message SearchResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchResponse} SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchResponse message.
         * @function verify
         * @memberof petajournal.SearchResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.results != null && message.hasOwnProperty("results")) {
                if (!Array.isArray(message.results))
                    return "results: array expected";
                for (let i = 0; i < message.results.length; ++i) {
                    let error = $root.petajournal.SearchResult.verify(message.results[i]);
                    if (error)
                        return "results." + error;
                }
            }
            if (message.totalHits != null && message.hasOwnProperty("totalHits"))
                if (!$util.isInteger(message.totalHits))
                    return "totalHits: integer expected";
            return null;
        };

        /**
         * Creates a SearchResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchResponse} SearchResponse
         */
        SearchResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchResponse)
                return object;
            let message = new $root.petajournal.SearchResponse();
            if (object.results) {
                if (!Array.isArray(object.results))
                    throw TypeError(".petajournal.SearchResponse.results: array expected");
                message.results = [];
                for (let i = 0; i < object.results.length; ++i) {
                    if (typeof object.results[i] !== "object")
                        throw TypeError(".petajournal.SearchResponse.results: object expected");
                    message.results[i] = $root.petajournal.SearchResult.fromObject(object.results[i]);
                }
            }
            if (object.totalHits != null)
                message.totalHits = object.totalHits | 0;
            return message;
        };

        /**
         * Creates a plain object from a SearchResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchResponse
         * @static
         * @param {petajournal.SearchResponse} message SearchResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.results = [];
            if (options.defaults)
                object.totalHits = 0;
            if (message.results && message.results.length) {
                object.results = [];
                for (let j = 0; j < message.results.length; ++j)
                    object.results[j] = $root.petajournal.SearchResult.toObject(message.results[j], options);
            }
            if (message.totalHits != null && message.hasOwnProperty("totalHits"))
                object.totalHits = message.totalHits;
            return object;
        };

        /**
         * Converts this SearchResponse to JSON.
         * @function toJSON
         * @memberof petajournal.SearchResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchResult message, length delimited. Does not implicitly {@link petajournal.SearchResult.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchResult
         * @static
         * @param {petajournal.ISearchResult} message SearchResult message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchResult.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchResult message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchResult
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchResult} SearchResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchResult.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchResult message.
         * @function verify
         * @memberof petajournal.SearchResult
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchResult.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            if (message.pageTitle != null && message.hasOwnProperty("pageTitle"))
                if (!$util.isString(message.pageTitle))
                    return "pageTitle: string expected";
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                if (!$util.isString(message.blockId))
                    return "blockId: string expected";
            if (message.snippet != null && message.hasOwnProperty("snippet"))
                if (!$util.isString(message.snippet))
                    return "snippet: string expected";
            if (message.score != null && message.hasOwnProperty("score"))
                if (typeof message.score !== "number")
                    return "score: number expected";
            return null;
        };

        /**
         * Creates a SearchResult message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchResult
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchResult} SearchResult
         */
        SearchResult.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchResult)
                return object;
            let message = new $root.petajournal.SearchResult();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            if (object.pageTitle != null)
                message.pageTitle = String(object.pageTitle);
            if (object.blockId != null)
                message.blockId = String(object.blockId);
            if (object.snippet != null)
                message.snippet = String(object.snippet);
            if (object.score != null)
                message.score = Number(object.score);
            return message;
        };

        /**
         * Creates a plain object from a SearchResult message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchResult
         * @static
         * @param {petajournal.SearchResult} message SearchResult
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchResult.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.pageId = "";
                object.pageTitle = "";
                object.blockId = "";
                object.snippet = "";
                object.score = 0;
            }
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            if (message.pageTitle != null && message.hasOwnProperty("pageTitle"))
                object.pageTitle = message.pageTitle;
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                object.blockId = message.blockId;
            if (message.snippet != null && message.hasOwnProperty("snippet"))
                object.snippet = message.snippet;
            if (message.score != null && message.hasOwnProperty("score"))
                object.score = options.json && !isFinite(message.score) ? String(message.score) : message.score;
            return object;
        };

        /**
         * Converts this SearchResult to JSON.
         * @function toJSON
         * @memberof petajournal.SearchResult
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchResult.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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

    petajournal.BookmarkGroup = (function() {

        /**
         * Properties of a BookmarkGroup.
         * @memberof petajournal
         * @interface IBookmarkGroup
         * @property {string|null} [id] BookmarkGroup id
         * @property {string|null} [name] BookmarkGroup name
         * @property {string|null} [sortOrder] BookmarkGroup sortOrder
         * @property {petajournal.ITimestamp|null} [createdAt] BookmarkGroup createdAt
         */

        /**
         * Constructs a new BookmarkGroup.
         * @memberof petajournal
         * @classdesc Represents a BookmarkGroup.
         * @implements IBookmarkGroup
         * @constructor
         * @param {petajournal.IBookmarkGroup=} [properties] Properties to set
         */
        function BookmarkGroup(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * BookmarkGroup id.
         * @member {string} id
         * @memberof petajournal.BookmarkGroup
         * @instance
         */
        BookmarkGroup.prototype.id = "";

        /**
         * BookmarkGroup name.
         * @member {string} name
         * @memberof petajournal.BookmarkGroup
         * @instance
         */
        BookmarkGroup.prototype.name = "";

        /**
         * BookmarkGroup sortOrder.
         * @member {string} sortOrder
         * @memberof petajournal.BookmarkGroup
         * @instance
         */
        BookmarkGroup.prototype.sortOrder = "";

        /**
         * BookmarkGroup createdAt.
         * @member {petajournal.ITimestamp|null|undefined} createdAt
         * @memberof petajournal.BookmarkGroup
         * @instance
         */
        BookmarkGroup.prototype.createdAt = null;

        /**
         * Creates a new BookmarkGroup instance using the specified properties.
         * @function create
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {petajournal.IBookmarkGroup=} [properties] Properties to set
         * @returns {petajournal.BookmarkGroup} BookmarkGroup instance
         */
        BookmarkGroup.create = function create(properties) {
            return new BookmarkGroup(properties);
        };

        /**
         * Encodes the specified BookmarkGroup message. Does not implicitly {@link petajournal.BookmarkGroup.verify|verify} messages.
         * @function encode
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {petajournal.IBookmarkGroup} message BookmarkGroup message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        BookmarkGroup.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.id);
            if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.name);
            if (message.sortOrder != null && Object.hasOwnProperty.call(message, "sortOrder"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.sortOrder);
            if (message.createdAt != null && Object.hasOwnProperty.call(message, "createdAt"))
                $root.petajournal.Timestamp.encode(message.createdAt, writer.uint32(/* id 4, wireType 2 =*/34).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified BookmarkGroup message, length delimited. Does not implicitly {@link petajournal.BookmarkGroup.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {petajournal.IBookmarkGroup} message BookmarkGroup message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        BookmarkGroup.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a BookmarkGroup message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.BookmarkGroup} BookmarkGroup
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        BookmarkGroup.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.BookmarkGroup();
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
                        message.name = reader.string();
                        break;
                    }
                case 3: {
                        message.sortOrder = reader.string();
                        break;
                    }
                case 4: {
                        message.createdAt = $root.petajournal.Timestamp.decode(reader, reader.uint32());
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
         * Decodes a BookmarkGroup message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.BookmarkGroup} BookmarkGroup
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        BookmarkGroup.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a BookmarkGroup message.
         * @function verify
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        BookmarkGroup.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.id != null && message.hasOwnProperty("id"))
                if (!$util.isString(message.id))
                    return "id: string expected";
            if (message.name != null && message.hasOwnProperty("name"))
                if (!$util.isString(message.name))
                    return "name: string expected";
            if (message.sortOrder != null && message.hasOwnProperty("sortOrder"))
                if (!$util.isString(message.sortOrder))
                    return "sortOrder: string expected";
            if (message.createdAt != null && message.hasOwnProperty("createdAt")) {
                let error = $root.petajournal.Timestamp.verify(message.createdAt);
                if (error)
                    return "createdAt." + error;
            }
            return null;
        };

        /**
         * Creates a BookmarkGroup message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.BookmarkGroup} BookmarkGroup
         */
        BookmarkGroup.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.BookmarkGroup)
                return object;
            let message = new $root.petajournal.BookmarkGroup();
            if (object.id != null)
                message.id = String(object.id);
            if (object.name != null)
                message.name = String(object.name);
            if (object.sortOrder != null)
                message.sortOrder = String(object.sortOrder);
            if (object.createdAt != null) {
                if (typeof object.createdAt !== "object")
                    throw TypeError(".petajournal.BookmarkGroup.createdAt: object expected");
                message.createdAt = $root.petajournal.Timestamp.fromObject(object.createdAt);
            }
            return message;
        };

        /**
         * Creates a plain object from a BookmarkGroup message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {petajournal.BookmarkGroup} message BookmarkGroup
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        BookmarkGroup.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.id = "";
                object.name = "";
                object.sortOrder = "";
                object.createdAt = null;
            }
            if (message.id != null && message.hasOwnProperty("id"))
                object.id = message.id;
            if (message.name != null && message.hasOwnProperty("name"))
                object.name = message.name;
            if (message.sortOrder != null && message.hasOwnProperty("sortOrder"))
                object.sortOrder = message.sortOrder;
            if (message.createdAt != null && message.hasOwnProperty("createdAt"))
                object.createdAt = $root.petajournal.Timestamp.toObject(message.createdAt, options);
            return object;
        };

        /**
         * Converts this BookmarkGroup to JSON.
         * @function toJSON
         * @memberof petajournal.BookmarkGroup
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        BookmarkGroup.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for BookmarkGroup
         * @function getTypeUrl
         * @memberof petajournal.BookmarkGroup
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        BookmarkGroup.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.BookmarkGroup";
        };

        return BookmarkGroup;
    })();

    petajournal.Bookmark = (function() {

        /**
         * Properties of a Bookmark.
         * @memberof petajournal
         * @interface IBookmark
         * @property {string|null} [id] Bookmark id
         * @property {string|null} [groupId] Bookmark groupId
         * @property {string|null} [targetType] Bookmark targetType
         * @property {string|null} [targetId] Bookmark targetId
         * @property {string|null} [sortOrder] Bookmark sortOrder
         * @property {petajournal.ITimestamp|null} [createdAt] Bookmark createdAt
         * @property {petajournal.IPage|null} [targetPage] Bookmark targetPage
         */

        /**
         * Constructs a new Bookmark.
         * @memberof petajournal
         * @classdesc Represents a Bookmark.
         * @implements IBookmark
         * @constructor
         * @param {petajournal.IBookmark=} [properties] Properties to set
         */
        function Bookmark(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Bookmark id.
         * @member {string} id
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.id = "";

        /**
         * Bookmark groupId.
         * @member {string} groupId
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.groupId = "";

        /**
         * Bookmark targetType.
         * @member {string} targetType
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.targetType = "";

        /**
         * Bookmark targetId.
         * @member {string} targetId
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.targetId = "";

        /**
         * Bookmark sortOrder.
         * @member {string} sortOrder
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.sortOrder = "";

        /**
         * Bookmark createdAt.
         * @member {petajournal.ITimestamp|null|undefined} createdAt
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.createdAt = null;

        /**
         * Bookmark targetPage.
         * @member {petajournal.IPage|null|undefined} targetPage
         * @memberof petajournal.Bookmark
         * @instance
         */
        Bookmark.prototype.targetPage = null;

        /**
         * Creates a new Bookmark instance using the specified properties.
         * @function create
         * @memberof petajournal.Bookmark
         * @static
         * @param {petajournal.IBookmark=} [properties] Properties to set
         * @returns {petajournal.Bookmark} Bookmark instance
         */
        Bookmark.create = function create(properties) {
            return new Bookmark(properties);
        };

        /**
         * Encodes the specified Bookmark message. Does not implicitly {@link petajournal.Bookmark.verify|verify} messages.
         * @function encode
         * @memberof petajournal.Bookmark
         * @static
         * @param {petajournal.IBookmark} message Bookmark message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Bookmark.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.id);
            if (message.groupId != null && Object.hasOwnProperty.call(message, "groupId"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.groupId);
            if (message.targetType != null && Object.hasOwnProperty.call(message, "targetType"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.targetType);
            if (message.targetId != null && Object.hasOwnProperty.call(message, "targetId"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.targetId);
            if (message.sortOrder != null && Object.hasOwnProperty.call(message, "sortOrder"))
                writer.uint32(/* id 5, wireType 2 =*/42).string(message.sortOrder);
            if (message.createdAt != null && Object.hasOwnProperty.call(message, "createdAt"))
                $root.petajournal.Timestamp.encode(message.createdAt, writer.uint32(/* id 6, wireType 2 =*/50).fork()).ldelim();
            if (message.targetPage != null && Object.hasOwnProperty.call(message, "targetPage"))
                $root.petajournal.Page.encode(message.targetPage, writer.uint32(/* id 7, wireType 2 =*/58).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified Bookmark message, length delimited. Does not implicitly {@link petajournal.Bookmark.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.Bookmark
         * @static
         * @param {petajournal.IBookmark} message Bookmark message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Bookmark.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a Bookmark message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.Bookmark
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.Bookmark} Bookmark
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Bookmark.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.Bookmark();
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
                        message.groupId = reader.string();
                        break;
                    }
                case 3: {
                        message.targetType = reader.string();
                        break;
                    }
                case 4: {
                        message.targetId = reader.string();
                        break;
                    }
                case 5: {
                        message.sortOrder = reader.string();
                        break;
                    }
                case 6: {
                        message.createdAt = $root.petajournal.Timestamp.decode(reader, reader.uint32());
                        break;
                    }
                case 7: {
                        message.targetPage = $root.petajournal.Page.decode(reader, reader.uint32());
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
         * Decodes a Bookmark message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.Bookmark
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.Bookmark} Bookmark
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Bookmark.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Bookmark message.
         * @function verify
         * @memberof petajournal.Bookmark
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Bookmark.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.id != null && message.hasOwnProperty("id"))
                if (!$util.isString(message.id))
                    return "id: string expected";
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                if (!$util.isString(message.groupId))
                    return "groupId: string expected";
            if (message.targetType != null && message.hasOwnProperty("targetType"))
                if (!$util.isString(message.targetType))
                    return "targetType: string expected";
            if (message.targetId != null && message.hasOwnProperty("targetId"))
                if (!$util.isString(message.targetId))
                    return "targetId: string expected";
            if (message.sortOrder != null && message.hasOwnProperty("sortOrder"))
                if (!$util.isString(message.sortOrder))
                    return "sortOrder: string expected";
            if (message.createdAt != null && message.hasOwnProperty("createdAt")) {
                let error = $root.petajournal.Timestamp.verify(message.createdAt);
                if (error)
                    return "createdAt." + error;
            }
            if (message.targetPage != null && message.hasOwnProperty("targetPage")) {
                let error = $root.petajournal.Page.verify(message.targetPage);
                if (error)
                    return "targetPage." + error;
            }
            return null;
        };

        /**
         * Creates a Bookmark message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.Bookmark
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.Bookmark} Bookmark
         */
        Bookmark.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.Bookmark)
                return object;
            let message = new $root.petajournal.Bookmark();
            if (object.id != null)
                message.id = String(object.id);
            if (object.groupId != null)
                message.groupId = String(object.groupId);
            if (object.targetType != null)
                message.targetType = String(object.targetType);
            if (object.targetId != null)
                message.targetId = String(object.targetId);
            if (object.sortOrder != null)
                message.sortOrder = String(object.sortOrder);
            if (object.createdAt != null) {
                if (typeof object.createdAt !== "object")
                    throw TypeError(".petajournal.Bookmark.createdAt: object expected");
                message.createdAt = $root.petajournal.Timestamp.fromObject(object.createdAt);
            }
            if (object.targetPage != null) {
                if (typeof object.targetPage !== "object")
                    throw TypeError(".petajournal.Bookmark.targetPage: object expected");
                message.targetPage = $root.petajournal.Page.fromObject(object.targetPage);
            }
            return message;
        };

        /**
         * Creates a plain object from a Bookmark message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.Bookmark
         * @static
         * @param {petajournal.Bookmark} message Bookmark
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Bookmark.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.id = "";
                object.groupId = "";
                object.targetType = "";
                object.targetId = "";
                object.sortOrder = "";
                object.createdAt = null;
                object.targetPage = null;
            }
            if (message.id != null && message.hasOwnProperty("id"))
                object.id = message.id;
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                object.groupId = message.groupId;
            if (message.targetType != null && message.hasOwnProperty("targetType"))
                object.targetType = message.targetType;
            if (message.targetId != null && message.hasOwnProperty("targetId"))
                object.targetId = message.targetId;
            if (message.sortOrder != null && message.hasOwnProperty("sortOrder"))
                object.sortOrder = message.sortOrder;
            if (message.createdAt != null && message.hasOwnProperty("createdAt"))
                object.createdAt = $root.petajournal.Timestamp.toObject(message.createdAt, options);
            if (message.targetPage != null && message.hasOwnProperty("targetPage"))
                object.targetPage = $root.petajournal.Page.toObject(message.targetPage, options);
            return object;
        };

        /**
         * Converts this Bookmark to JSON.
         * @function toJSON
         * @memberof petajournal.Bookmark
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Bookmark.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Bookmark
         * @function getTypeUrl
         * @memberof petajournal.Bookmark
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Bookmark.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.Bookmark";
        };

        return Bookmark;
    })();

    petajournal.ApiRequest = (function() {

        /**
         * Properties of an ApiRequest.
         * @memberof petajournal
         * @interface IApiRequest
         * @property {string|null} [requestId] ApiRequest requestId
         * @property {petajournal.IGetPageByIdRequest|null} [getPageById] ApiRequest getPageById
         * @property {petajournal.IGetOrCreateDocRequest|null} [getOrCreateDoc] ApiRequest getOrCreateDoc
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
         * @property {petajournal.IGetOrCreateJournalRequest|null} [getOrCreateJournal] ApiRequest getOrCreateJournal
         * @property {petajournal.IExportTreeMarkdownRequest|null} [exportTreeMarkdown] ApiRequest exportTreeMarkdown
         * @property {petajournal.IImportTreeMarkdownRequest|null} [importTreeMarkdown] ApiRequest importTreeMarkdown
         * @property {petajournal.IGetBookmarksRequest|null} [getBookmarks] ApiRequest getBookmarks
         * @property {petajournal.ICreateBookmarkGroupRequest|null} [createBookmarkGroup] ApiRequest createBookmarkGroup
         * @property {petajournal.ICreateBookmarkRequest|null} [createBookmark] ApiRequest createBookmark
         * @property {petajournal.IDeleteBookmarkRequest|null} [deleteBookmark] ApiRequest deleteBookmark
         * @property {petajournal.IDeleteBookmarkGroupRequest|null} [deleteBookmarkGroup] ApiRequest deleteBookmarkGroup
         * @property {petajournal.IMoveBookmarkRequest|null} [moveBookmark] ApiRequest moveBookmark
         * @property {petajournal.IMoveBookmarkGroupRequest|null} [moveBookmarkGroup] ApiRequest moveBookmarkGroup
         * @property {petajournal.IUpdateBookmarkGroupRequest|null} [updateBookmarkGroup] ApiRequest updateBookmarkGroup
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
         * ApiRequest getPageById.
         * @member {petajournal.IGetPageByIdRequest|null|undefined} getPageById
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getPageById = null;

        /**
         * ApiRequest getOrCreateDoc.
         * @member {petajournal.IGetOrCreateDocRequest|null|undefined} getOrCreateDoc
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getOrCreateDoc = null;

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
         * ApiRequest getOrCreateJournal.
         * @member {petajournal.IGetOrCreateJournalRequest|null|undefined} getOrCreateJournal
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getOrCreateJournal = null;

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

        /**
         * ApiRequest getBookmarks.
         * @member {petajournal.IGetBookmarksRequest|null|undefined} getBookmarks
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.getBookmarks = null;

        /**
         * ApiRequest createBookmarkGroup.
         * @member {petajournal.ICreateBookmarkGroupRequest|null|undefined} createBookmarkGroup
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.createBookmarkGroup = null;

        /**
         * ApiRequest createBookmark.
         * @member {petajournal.ICreateBookmarkRequest|null|undefined} createBookmark
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.createBookmark = null;

        /**
         * ApiRequest deleteBookmark.
         * @member {petajournal.IDeleteBookmarkRequest|null|undefined} deleteBookmark
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.deleteBookmark = null;

        /**
         * ApiRequest deleteBookmarkGroup.
         * @member {petajournal.IDeleteBookmarkGroupRequest|null|undefined} deleteBookmarkGroup
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.deleteBookmarkGroup = null;

        /**
         * ApiRequest moveBookmark.
         * @member {petajournal.IMoveBookmarkRequest|null|undefined} moveBookmark
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.moveBookmark = null;

        /**
         * ApiRequest moveBookmarkGroup.
         * @member {petajournal.IMoveBookmarkGroupRequest|null|undefined} moveBookmarkGroup
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.moveBookmarkGroup = null;

        /**
         * ApiRequest updateBookmarkGroup.
         * @member {petajournal.IUpdateBookmarkGroupRequest|null|undefined} updateBookmarkGroup
         * @memberof petajournal.ApiRequest
         * @instance
         */
        ApiRequest.prototype.updateBookmarkGroup = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        /**
         * ApiRequest payload.
         * @member {"getPageById"|"getOrCreateDoc"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getBlocksByStatus"|"getConfig"|"updateConfig"|"getOrCreateJournal"|"exportTreeMarkdown"|"importTreeMarkdown"|"getBookmarks"|"createBookmarkGroup"|"createBookmark"|"deleteBookmark"|"deleteBookmarkGroup"|"moveBookmark"|"moveBookmarkGroup"|"updateBookmarkGroup"|undefined} payload
         * @memberof petajournal.ApiRequest
         * @instance
         */
        Object.defineProperty(ApiRequest.prototype, "payload", {
            get: $util.oneOfGetter($oneOfFields = ["getPageById", "getOrCreateDoc", "deletePage", "updatePage", "createBlock", "updateBlock", "moveBlocks", "deleteBlock", "updateBlockStatus", "searchPageNames", "searchFullText", "getRecentPages", "getBacklinks", "getMostRecentJournalBefore", "getBlocksByStatus", "getConfig", "updateConfig", "getOrCreateJournal", "exportTreeMarkdown", "importTreeMarkdown", "getBookmarks", "createBookmarkGroup", "createBookmark", "deleteBookmark", "deleteBookmarkGroup", "moveBookmark", "moveBookmarkGroup", "updateBookmarkGroup"]),
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
            if (message.getPageById != null && Object.hasOwnProperty.call(message, "getPageById"))
                $root.petajournal.GetPageByIdRequest.encode(message.getPageById, writer.uint32(/* id 10, wireType 2 =*/82).fork()).ldelim();
            if (message.getOrCreateDoc != null && Object.hasOwnProperty.call(message, "getOrCreateDoc"))
                $root.petajournal.GetOrCreateDocRequest.encode(message.getOrCreateDoc, writer.uint32(/* id 12, wireType 2 =*/98).fork()).ldelim();
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
            if (message.getOrCreateJournal != null && Object.hasOwnProperty.call(message, "getOrCreateJournal"))
                $root.petajournal.GetOrCreateJournalRequest.encode(message.getOrCreateJournal, writer.uint32(/* id 44, wireType 2 =*/354).fork()).ldelim();
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
            if (message.getBookmarks != null && Object.hasOwnProperty.call(message, "getBookmarks"))
                $root.petajournal.GetBookmarksRequest.encode(message.getBookmarks, writer.uint32(/* id 80, wireType 2 =*/642).fork()).ldelim();
            if (message.createBookmarkGroup != null && Object.hasOwnProperty.call(message, "createBookmarkGroup"))
                $root.petajournal.CreateBookmarkGroupRequest.encode(message.createBookmarkGroup, writer.uint32(/* id 81, wireType 2 =*/650).fork()).ldelim();
            if (message.createBookmark != null && Object.hasOwnProperty.call(message, "createBookmark"))
                $root.petajournal.CreateBookmarkRequest.encode(message.createBookmark, writer.uint32(/* id 82, wireType 2 =*/658).fork()).ldelim();
            if (message.deleteBookmark != null && Object.hasOwnProperty.call(message, "deleteBookmark"))
                $root.petajournal.DeleteBookmarkRequest.encode(message.deleteBookmark, writer.uint32(/* id 83, wireType 2 =*/666).fork()).ldelim();
            if (message.deleteBookmarkGroup != null && Object.hasOwnProperty.call(message, "deleteBookmarkGroup"))
                $root.petajournal.DeleteBookmarkGroupRequest.encode(message.deleteBookmarkGroup, writer.uint32(/* id 84, wireType 2 =*/674).fork()).ldelim();
            if (message.moveBookmark != null && Object.hasOwnProperty.call(message, "moveBookmark"))
                $root.petajournal.MoveBookmarkRequest.encode(message.moveBookmark, writer.uint32(/* id 85, wireType 2 =*/682).fork()).ldelim();
            if (message.moveBookmarkGroup != null && Object.hasOwnProperty.call(message, "moveBookmarkGroup"))
                $root.petajournal.MoveBookmarkGroupRequest.encode(message.moveBookmarkGroup, writer.uint32(/* id 86, wireType 2 =*/690).fork()).ldelim();
            if (message.updateBookmarkGroup != null && Object.hasOwnProperty.call(message, "updateBookmarkGroup"))
                $root.petajournal.UpdateBookmarkGroupRequest.encode(message.updateBookmarkGroup, writer.uint32(/* id 87, wireType 2 =*/698).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified ApiRequest message, length delimited. Does not implicitly {@link petajournal.ApiRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ApiRequest
         * @static
         * @param {petajournal.IApiRequest} message ApiRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ApiRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                        message.getPageById = $root.petajournal.GetPageByIdRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 12: {
                        message.getOrCreateDoc = $root.petajournal.GetOrCreateDocRequest.decode(reader, reader.uint32());
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
                        message.getOrCreateJournal = $root.petajournal.GetOrCreateJournalRequest.decode(reader, reader.uint32());
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
                case 80: {
                        message.getBookmarks = $root.petajournal.GetBookmarksRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 81: {
                        message.createBookmarkGroup = $root.petajournal.CreateBookmarkGroupRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 82: {
                        message.createBookmark = $root.petajournal.CreateBookmarkRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 83: {
                        message.deleteBookmark = $root.petajournal.DeleteBookmarkRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 84: {
                        message.deleteBookmarkGroup = $root.petajournal.DeleteBookmarkGroupRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 85: {
                        message.moveBookmark = $root.petajournal.MoveBookmarkRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 86: {
                        message.moveBookmarkGroup = $root.petajournal.MoveBookmarkGroupRequest.decode(reader, reader.uint32());
                        break;
                    }
                case 87: {
                        message.updateBookmarkGroup = $root.petajournal.UpdateBookmarkGroupRequest.decode(reader, reader.uint32());
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
         * Decodes an ApiRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ApiRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ApiRequest} ApiRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ApiRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an ApiRequest message.
         * @function verify
         * @memberof petajournal.ApiRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ApiRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            let properties = {};
            if (message.requestId != null && message.hasOwnProperty("requestId"))
                if (!$util.isString(message.requestId))
                    return "requestId: string expected";
            if (message.getPageById != null && message.hasOwnProperty("getPageById")) {
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetPageByIdRequest.verify(message.getPageById);
                    if (error)
                        return "getPageById." + error;
                }
            }
            if (message.getOrCreateDoc != null && message.hasOwnProperty("getOrCreateDoc")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetOrCreateDocRequest.verify(message.getOrCreateDoc);
                    if (error)
                        return "getOrCreateDoc." + error;
                }
            }
            if (message.deletePage != null && message.hasOwnProperty("deletePage")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeletePageRequest.verify(message.deletePage);
                    if (error)
                        return "deletePage." + error;
                }
            }
            if (message.updatePage != null && message.hasOwnProperty("updatePage")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdatePageRequest.verify(message.updatePage);
                    if (error)
                        return "updatePage." + error;
                }
            }
            if (message.createBlock != null && message.hasOwnProperty("createBlock")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.CreateBlockRequest.verify(message.createBlock);
                    if (error)
                        return "createBlock." + error;
                }
            }
            if (message.updateBlock != null && message.hasOwnProperty("updateBlock")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateBlockRequest.verify(message.updateBlock);
                    if (error)
                        return "updateBlock." + error;
                }
            }
            if (message.moveBlocks != null && message.hasOwnProperty("moveBlocks")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.MoveBlocksRequest.verify(message.moveBlocks);
                    if (error)
                        return "moveBlocks." + error;
                }
            }
            if (message.deleteBlock != null && message.hasOwnProperty("deleteBlock")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeleteBlockRequest.verify(message.deleteBlock);
                    if (error)
                        return "deleteBlock." + error;
                }
            }
            if (message.updateBlockStatus != null && message.hasOwnProperty("updateBlockStatus")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateBlockStatusRequest.verify(message.updateBlockStatus);
                    if (error)
                        return "updateBlockStatus." + error;
                }
            }
            if (message.searchPageNames != null && message.hasOwnProperty("searchPageNames")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.SearchPageNamesRequest.verify(message.searchPageNames);
                    if (error)
                        return "searchPageNames." + error;
                }
            }
            if (message.searchFullText != null && message.hasOwnProperty("searchFullText")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.SearchFullTextRequest.verify(message.searchFullText);
                    if (error)
                        return "searchFullText." + error;
                }
            }
            if (message.getRecentPages != null && message.hasOwnProperty("getRecentPages")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetRecentPagesRequest.verify(message.getRecentPages);
                    if (error)
                        return "getRecentPages." + error;
                }
            }
            if (message.getBacklinks != null && message.hasOwnProperty("getBacklinks")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetBacklinksRequest.verify(message.getBacklinks);
                    if (error)
                        return "getBacklinks." + error;
                }
            }
            if (message.getMostRecentJournalBefore != null && message.hasOwnProperty("getMostRecentJournalBefore")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetMostRecentJournalBeforeRequest.verify(message.getMostRecentJournalBefore);
                    if (error)
                        return "getMostRecentJournalBefore." + error;
                }
            }
            if (message.getBlocksByStatus != null && message.hasOwnProperty("getBlocksByStatus")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetBlocksByStatusRequest.verify(message.getBlocksByStatus);
                    if (error)
                        return "getBlocksByStatus." + error;
                }
            }
            if (message.getConfig != null && message.hasOwnProperty("getConfig")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetConfigRequest.verify(message.getConfig);
                    if (error)
                        return "getConfig." + error;
                }
            }
            if (message.updateConfig != null && message.hasOwnProperty("updateConfig")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateConfigRequest.verify(message.updateConfig);
                    if (error)
                        return "updateConfig." + error;
                }
            }
            if (message.getOrCreateJournal != null && message.hasOwnProperty("getOrCreateJournal")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetOrCreateJournalRequest.verify(message.getOrCreateJournal);
                    if (error)
                        return "getOrCreateJournal." + error;
                }
            }
            if (message.exportTreeMarkdown != null && message.hasOwnProperty("exportTreeMarkdown")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.ExportTreeMarkdownRequest.verify(message.exportTreeMarkdown);
                    if (error)
                        return "exportTreeMarkdown." + error;
                }
            }
            if (message.importTreeMarkdown != null && message.hasOwnProperty("importTreeMarkdown")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.ImportTreeMarkdownRequest.verify(message.importTreeMarkdown);
                    if (error)
                        return "importTreeMarkdown." + error;
                }
            }
            if (message.getBookmarks != null && message.hasOwnProperty("getBookmarks")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetBookmarksRequest.verify(message.getBookmarks);
                    if (error)
                        return "getBookmarks." + error;
                }
            }
            if (message.createBookmarkGroup != null && message.hasOwnProperty("createBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.CreateBookmarkGroupRequest.verify(message.createBookmarkGroup);
                    if (error)
                        return "createBookmarkGroup." + error;
                }
            }
            if (message.createBookmark != null && message.hasOwnProperty("createBookmark")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.CreateBookmarkRequest.verify(message.createBookmark);
                    if (error)
                        return "createBookmark." + error;
                }
            }
            if (message.deleteBookmark != null && message.hasOwnProperty("deleteBookmark")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeleteBookmarkRequest.verify(message.deleteBookmark);
                    if (error)
                        return "deleteBookmark." + error;
                }
            }
            if (message.deleteBookmarkGroup != null && message.hasOwnProperty("deleteBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeleteBookmarkGroupRequest.verify(message.deleteBookmarkGroup);
                    if (error)
                        return "deleteBookmarkGroup." + error;
                }
            }
            if (message.moveBookmark != null && message.hasOwnProperty("moveBookmark")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.MoveBookmarkRequest.verify(message.moveBookmark);
                    if (error)
                        return "moveBookmark." + error;
                }
            }
            if (message.moveBookmarkGroup != null && message.hasOwnProperty("moveBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.MoveBookmarkGroupRequest.verify(message.moveBookmarkGroup);
                    if (error)
                        return "moveBookmarkGroup." + error;
                }
            }
            if (message.updateBookmarkGroup != null && message.hasOwnProperty("updateBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateBookmarkGroupRequest.verify(message.updateBookmarkGroup);
                    if (error)
                        return "updateBookmarkGroup." + error;
                }
            }
            return null;
        };

        /**
         * Creates an ApiRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ApiRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ApiRequest} ApiRequest
         */
        ApiRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ApiRequest)
                return object;
            let message = new $root.petajournal.ApiRequest();
            if (object.requestId != null)
                message.requestId = String(object.requestId);
            if (object.getPageById != null) {
                if (typeof object.getPageById !== "object")
                    throw TypeError(".petajournal.ApiRequest.getPageById: object expected");
                message.getPageById = $root.petajournal.GetPageByIdRequest.fromObject(object.getPageById);
            }
            if (object.getOrCreateDoc != null) {
                if (typeof object.getOrCreateDoc !== "object")
                    throw TypeError(".petajournal.ApiRequest.getOrCreateDoc: object expected");
                message.getOrCreateDoc = $root.petajournal.GetOrCreateDocRequest.fromObject(object.getOrCreateDoc);
            }
            if (object.deletePage != null) {
                if (typeof object.deletePage !== "object")
                    throw TypeError(".petajournal.ApiRequest.deletePage: object expected");
                message.deletePage = $root.petajournal.DeletePageRequest.fromObject(object.deletePage);
            }
            if (object.updatePage != null) {
                if (typeof object.updatePage !== "object")
                    throw TypeError(".petajournal.ApiRequest.updatePage: object expected");
                message.updatePage = $root.petajournal.UpdatePageRequest.fromObject(object.updatePage);
            }
            if (object.createBlock != null) {
                if (typeof object.createBlock !== "object")
                    throw TypeError(".petajournal.ApiRequest.createBlock: object expected");
                message.createBlock = $root.petajournal.CreateBlockRequest.fromObject(object.createBlock);
            }
            if (object.updateBlock != null) {
                if (typeof object.updateBlock !== "object")
                    throw TypeError(".petajournal.ApiRequest.updateBlock: object expected");
                message.updateBlock = $root.petajournal.UpdateBlockRequest.fromObject(object.updateBlock);
            }
            if (object.moveBlocks != null) {
                if (typeof object.moveBlocks !== "object")
                    throw TypeError(".petajournal.ApiRequest.moveBlocks: object expected");
                message.moveBlocks = $root.petajournal.MoveBlocksRequest.fromObject(object.moveBlocks);
            }
            if (object.deleteBlock != null) {
                if (typeof object.deleteBlock !== "object")
                    throw TypeError(".petajournal.ApiRequest.deleteBlock: object expected");
                message.deleteBlock = $root.petajournal.DeleteBlockRequest.fromObject(object.deleteBlock);
            }
            if (object.updateBlockStatus != null) {
                if (typeof object.updateBlockStatus !== "object")
                    throw TypeError(".petajournal.ApiRequest.updateBlockStatus: object expected");
                message.updateBlockStatus = $root.petajournal.UpdateBlockStatusRequest.fromObject(object.updateBlockStatus);
            }
            if (object.searchPageNames != null) {
                if (typeof object.searchPageNames !== "object")
                    throw TypeError(".petajournal.ApiRequest.searchPageNames: object expected");
                message.searchPageNames = $root.petajournal.SearchPageNamesRequest.fromObject(object.searchPageNames);
            }
            if (object.searchFullText != null) {
                if (typeof object.searchFullText !== "object")
                    throw TypeError(".petajournal.ApiRequest.searchFullText: object expected");
                message.searchFullText = $root.petajournal.SearchFullTextRequest.fromObject(object.searchFullText);
            }
            if (object.getRecentPages != null) {
                if (typeof object.getRecentPages !== "object")
                    throw TypeError(".petajournal.ApiRequest.getRecentPages: object expected");
                message.getRecentPages = $root.petajournal.GetRecentPagesRequest.fromObject(object.getRecentPages);
            }
            if (object.getBacklinks != null) {
                if (typeof object.getBacklinks !== "object")
                    throw TypeError(".petajournal.ApiRequest.getBacklinks: object expected");
                message.getBacklinks = $root.petajournal.GetBacklinksRequest.fromObject(object.getBacklinks);
            }
            if (object.getMostRecentJournalBefore != null) {
                if (typeof object.getMostRecentJournalBefore !== "object")
                    throw TypeError(".petajournal.ApiRequest.getMostRecentJournalBefore: object expected");
                message.getMostRecentJournalBefore = $root.petajournal.GetMostRecentJournalBeforeRequest.fromObject(object.getMostRecentJournalBefore);
            }
            if (object.getBlocksByStatus != null) {
                if (typeof object.getBlocksByStatus !== "object")
                    throw TypeError(".petajournal.ApiRequest.getBlocksByStatus: object expected");
                message.getBlocksByStatus = $root.petajournal.GetBlocksByStatusRequest.fromObject(object.getBlocksByStatus);
            }
            if (object.getConfig != null) {
                if (typeof object.getConfig !== "object")
                    throw TypeError(".petajournal.ApiRequest.getConfig: object expected");
                message.getConfig = $root.petajournal.GetConfigRequest.fromObject(object.getConfig);
            }
            if (object.updateConfig != null) {
                if (typeof object.updateConfig !== "object")
                    throw TypeError(".petajournal.ApiRequest.updateConfig: object expected");
                message.updateConfig = $root.petajournal.UpdateConfigRequest.fromObject(object.updateConfig);
            }
            if (object.getOrCreateJournal != null) {
                if (typeof object.getOrCreateJournal !== "object")
                    throw TypeError(".petajournal.ApiRequest.getOrCreateJournal: object expected");
                message.getOrCreateJournal = $root.petajournal.GetOrCreateJournalRequest.fromObject(object.getOrCreateJournal);
            }
            if (object.exportTreeMarkdown != null) {
                if (typeof object.exportTreeMarkdown !== "object")
                    throw TypeError(".petajournal.ApiRequest.exportTreeMarkdown: object expected");
                message.exportTreeMarkdown = $root.petajournal.ExportTreeMarkdownRequest.fromObject(object.exportTreeMarkdown);
            }
            if (object.importTreeMarkdown != null) {
                if (typeof object.importTreeMarkdown !== "object")
                    throw TypeError(".petajournal.ApiRequest.importTreeMarkdown: object expected");
                message.importTreeMarkdown = $root.petajournal.ImportTreeMarkdownRequest.fromObject(object.importTreeMarkdown);
            }
            if (object.getBookmarks != null) {
                if (typeof object.getBookmarks !== "object")
                    throw TypeError(".petajournal.ApiRequest.getBookmarks: object expected");
                message.getBookmarks = $root.petajournal.GetBookmarksRequest.fromObject(object.getBookmarks);
            }
            if (object.createBookmarkGroup != null) {
                if (typeof object.createBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiRequest.createBookmarkGroup: object expected");
                message.createBookmarkGroup = $root.petajournal.CreateBookmarkGroupRequest.fromObject(object.createBookmarkGroup);
            }
            if (object.createBookmark != null) {
                if (typeof object.createBookmark !== "object")
                    throw TypeError(".petajournal.ApiRequest.createBookmark: object expected");
                message.createBookmark = $root.petajournal.CreateBookmarkRequest.fromObject(object.createBookmark);
            }
            if (object.deleteBookmark != null) {
                if (typeof object.deleteBookmark !== "object")
                    throw TypeError(".petajournal.ApiRequest.deleteBookmark: object expected");
                message.deleteBookmark = $root.petajournal.DeleteBookmarkRequest.fromObject(object.deleteBookmark);
            }
            if (object.deleteBookmarkGroup != null) {
                if (typeof object.deleteBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiRequest.deleteBookmarkGroup: object expected");
                message.deleteBookmarkGroup = $root.petajournal.DeleteBookmarkGroupRequest.fromObject(object.deleteBookmarkGroup);
            }
            if (object.moveBookmark != null) {
                if (typeof object.moveBookmark !== "object")
                    throw TypeError(".petajournal.ApiRequest.moveBookmark: object expected");
                message.moveBookmark = $root.petajournal.MoveBookmarkRequest.fromObject(object.moveBookmark);
            }
            if (object.moveBookmarkGroup != null) {
                if (typeof object.moveBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiRequest.moveBookmarkGroup: object expected");
                message.moveBookmarkGroup = $root.petajournal.MoveBookmarkGroupRequest.fromObject(object.moveBookmarkGroup);
            }
            if (object.updateBookmarkGroup != null) {
                if (typeof object.updateBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiRequest.updateBookmarkGroup: object expected");
                message.updateBookmarkGroup = $root.petajournal.UpdateBookmarkGroupRequest.fromObject(object.updateBookmarkGroup);
            }
            return message;
        };

        /**
         * Creates a plain object from an ApiRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ApiRequest
         * @static
         * @param {petajournal.ApiRequest} message ApiRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ApiRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.requestId = "";
            if (message.requestId != null && message.hasOwnProperty("requestId"))
                object.requestId = message.requestId;
            if (message.getPageById != null && message.hasOwnProperty("getPageById")) {
                object.getPageById = $root.petajournal.GetPageByIdRequest.toObject(message.getPageById, options);
                if (options.oneofs)
                    object.payload = "getPageById";
            }
            if (message.getOrCreateDoc != null && message.hasOwnProperty("getOrCreateDoc")) {
                object.getOrCreateDoc = $root.petajournal.GetOrCreateDocRequest.toObject(message.getOrCreateDoc, options);
                if (options.oneofs)
                    object.payload = "getOrCreateDoc";
            }
            if (message.deletePage != null && message.hasOwnProperty("deletePage")) {
                object.deletePage = $root.petajournal.DeletePageRequest.toObject(message.deletePage, options);
                if (options.oneofs)
                    object.payload = "deletePage";
            }
            if (message.updatePage != null && message.hasOwnProperty("updatePage")) {
                object.updatePage = $root.petajournal.UpdatePageRequest.toObject(message.updatePage, options);
                if (options.oneofs)
                    object.payload = "updatePage";
            }
            if (message.createBlock != null && message.hasOwnProperty("createBlock")) {
                object.createBlock = $root.petajournal.CreateBlockRequest.toObject(message.createBlock, options);
                if (options.oneofs)
                    object.payload = "createBlock";
            }
            if (message.updateBlock != null && message.hasOwnProperty("updateBlock")) {
                object.updateBlock = $root.petajournal.UpdateBlockRequest.toObject(message.updateBlock, options);
                if (options.oneofs)
                    object.payload = "updateBlock";
            }
            if (message.moveBlocks != null && message.hasOwnProperty("moveBlocks")) {
                object.moveBlocks = $root.petajournal.MoveBlocksRequest.toObject(message.moveBlocks, options);
                if (options.oneofs)
                    object.payload = "moveBlocks";
            }
            if (message.deleteBlock != null && message.hasOwnProperty("deleteBlock")) {
                object.deleteBlock = $root.petajournal.DeleteBlockRequest.toObject(message.deleteBlock, options);
                if (options.oneofs)
                    object.payload = "deleteBlock";
            }
            if (message.updateBlockStatus != null && message.hasOwnProperty("updateBlockStatus")) {
                object.updateBlockStatus = $root.petajournal.UpdateBlockStatusRequest.toObject(message.updateBlockStatus, options);
                if (options.oneofs)
                    object.payload = "updateBlockStatus";
            }
            if (message.searchPageNames != null && message.hasOwnProperty("searchPageNames")) {
                object.searchPageNames = $root.petajournal.SearchPageNamesRequest.toObject(message.searchPageNames, options);
                if (options.oneofs)
                    object.payload = "searchPageNames";
            }
            if (message.searchFullText != null && message.hasOwnProperty("searchFullText")) {
                object.searchFullText = $root.petajournal.SearchFullTextRequest.toObject(message.searchFullText, options);
                if (options.oneofs)
                    object.payload = "searchFullText";
            }
            if (message.getRecentPages != null && message.hasOwnProperty("getRecentPages")) {
                object.getRecentPages = $root.petajournal.GetRecentPagesRequest.toObject(message.getRecentPages, options);
                if (options.oneofs)
                    object.payload = "getRecentPages";
            }
            if (message.getBacklinks != null && message.hasOwnProperty("getBacklinks")) {
                object.getBacklinks = $root.petajournal.GetBacklinksRequest.toObject(message.getBacklinks, options);
                if (options.oneofs)
                    object.payload = "getBacklinks";
            }
            if (message.getMostRecentJournalBefore != null && message.hasOwnProperty("getMostRecentJournalBefore")) {
                object.getMostRecentJournalBefore = $root.petajournal.GetMostRecentJournalBeforeRequest.toObject(message.getMostRecentJournalBefore, options);
                if (options.oneofs)
                    object.payload = "getMostRecentJournalBefore";
            }
            if (message.getOrCreateJournal != null && message.hasOwnProperty("getOrCreateJournal")) {
                object.getOrCreateJournal = $root.petajournal.GetOrCreateJournalRequest.toObject(message.getOrCreateJournal, options);
                if (options.oneofs)
                    object.payload = "getOrCreateJournal";
            }
            if (message.getBlocksByStatus != null && message.hasOwnProperty("getBlocksByStatus")) {
                object.getBlocksByStatus = $root.petajournal.GetBlocksByStatusRequest.toObject(message.getBlocksByStatus, options);
                if (options.oneofs)
                    object.payload = "getBlocksByStatus";
            }
            if (message.getConfig != null && message.hasOwnProperty("getConfig")) {
                object.getConfig = $root.petajournal.GetConfigRequest.toObject(message.getConfig, options);
                if (options.oneofs)
                    object.payload = "getConfig";
            }
            if (message.updateConfig != null && message.hasOwnProperty("updateConfig")) {
                object.updateConfig = $root.petajournal.UpdateConfigRequest.toObject(message.updateConfig, options);
                if (options.oneofs)
                    object.payload = "updateConfig";
            }
            if (message.exportTreeMarkdown != null && message.hasOwnProperty("exportTreeMarkdown")) {
                object.exportTreeMarkdown = $root.petajournal.ExportTreeMarkdownRequest.toObject(message.exportTreeMarkdown, options);
                if (options.oneofs)
                    object.payload = "exportTreeMarkdown";
            }
            if (message.importTreeMarkdown != null && message.hasOwnProperty("importTreeMarkdown")) {
                object.importTreeMarkdown = $root.petajournal.ImportTreeMarkdownRequest.toObject(message.importTreeMarkdown, options);
                if (options.oneofs)
                    object.payload = "importTreeMarkdown";
            }
            if (message.getBookmarks != null && message.hasOwnProperty("getBookmarks")) {
                object.getBookmarks = $root.petajournal.GetBookmarksRequest.toObject(message.getBookmarks, options);
                if (options.oneofs)
                    object.payload = "getBookmarks";
            }
            if (message.createBookmarkGroup != null && message.hasOwnProperty("createBookmarkGroup")) {
                object.createBookmarkGroup = $root.petajournal.CreateBookmarkGroupRequest.toObject(message.createBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "createBookmarkGroup";
            }
            if (message.createBookmark != null && message.hasOwnProperty("createBookmark")) {
                object.createBookmark = $root.petajournal.CreateBookmarkRequest.toObject(message.createBookmark, options);
                if (options.oneofs)
                    object.payload = "createBookmark";
            }
            if (message.deleteBookmark != null && message.hasOwnProperty("deleteBookmark")) {
                object.deleteBookmark = $root.petajournal.DeleteBookmarkRequest.toObject(message.deleteBookmark, options);
                if (options.oneofs)
                    object.payload = "deleteBookmark";
            }
            if (message.deleteBookmarkGroup != null && message.hasOwnProperty("deleteBookmarkGroup")) {
                object.deleteBookmarkGroup = $root.petajournal.DeleteBookmarkGroupRequest.toObject(message.deleteBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "deleteBookmarkGroup";
            }
            if (message.moveBookmark != null && message.hasOwnProperty("moveBookmark")) {
                object.moveBookmark = $root.petajournal.MoveBookmarkRequest.toObject(message.moveBookmark, options);
                if (options.oneofs)
                    object.payload = "moveBookmark";
            }
            if (message.moveBookmarkGroup != null && message.hasOwnProperty("moveBookmarkGroup")) {
                object.moveBookmarkGroup = $root.petajournal.MoveBookmarkGroupRequest.toObject(message.moveBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "moveBookmarkGroup";
            }
            if (message.updateBookmarkGroup != null && message.hasOwnProperty("updateBookmarkGroup")) {
                object.updateBookmarkGroup = $root.petajournal.UpdateBookmarkGroupRequest.toObject(message.updateBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "updateBookmarkGroup";
            }
            return object;
        };

        /**
         * Converts this ApiRequest to JSON.
         * @function toJSON
         * @memberof petajournal.ApiRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ApiRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * @property {petajournal.IGetPageByIdResponse|null} [getPageById] ApiResponse getPageById
         * @property {petajournal.IGetOrCreateDocResponse|null} [getOrCreateDoc] ApiResponse getOrCreateDoc
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
         * @property {petajournal.IGetOrCreateJournalResponse|null} [getOrCreateJournal] ApiResponse getOrCreateJournal
         * @property {petajournal.IGetBlocksByStatusResponse|null} [getBlocksByStatus] ApiResponse getBlocksByStatus
         * @property {petajournal.IGetConfigResponse|null} [getConfig] ApiResponse getConfig
         * @property {petajournal.IUpdateConfigResponse|null} [updateConfig] ApiResponse updateConfig
         * @property {petajournal.IExportTreeMarkdownResponse|null} [exportTreeMarkdown] ApiResponse exportTreeMarkdown
         * @property {petajournal.IImportTreeMarkdownResponse|null} [importTreeMarkdown] ApiResponse importTreeMarkdown
         * @property {petajournal.IGetBookmarksResponse|null} [getBookmarks] ApiResponse getBookmarks
         * @property {petajournal.ICreateBookmarkGroupResponse|null} [createBookmarkGroup] ApiResponse createBookmarkGroup
         * @property {petajournal.ICreateBookmarkResponse|null} [createBookmark] ApiResponse createBookmark
         * @property {petajournal.IDeleteBookmarkResponse|null} [deleteBookmark] ApiResponse deleteBookmark
         * @property {petajournal.IDeleteBookmarkGroupResponse|null} [deleteBookmarkGroup] ApiResponse deleteBookmarkGroup
         * @property {petajournal.IMoveBookmarkResponse|null} [moveBookmark] ApiResponse moveBookmark
         * @property {petajournal.IMoveBookmarkGroupResponse|null} [moveBookmarkGroup] ApiResponse moveBookmarkGroup
         * @property {petajournal.IUpdateBookmarkGroupResponse|null} [updateBookmarkGroup] ApiResponse updateBookmarkGroup
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
         * ApiResponse getPageById.
         * @member {petajournal.IGetPageByIdResponse|null|undefined} getPageById
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getPageById = null;

        /**
         * ApiResponse getOrCreateDoc.
         * @member {petajournal.IGetOrCreateDocResponse|null|undefined} getOrCreateDoc
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getOrCreateDoc = null;

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
         * ApiResponse getOrCreateJournal.
         * @member {petajournal.IGetOrCreateJournalResponse|null|undefined} getOrCreateJournal
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getOrCreateJournal = null;

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
         * ApiResponse getBookmarks.
         * @member {petajournal.IGetBookmarksResponse|null|undefined} getBookmarks
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.getBookmarks = null;

        /**
         * ApiResponse createBookmarkGroup.
         * @member {petajournal.ICreateBookmarkGroupResponse|null|undefined} createBookmarkGroup
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.createBookmarkGroup = null;

        /**
         * ApiResponse createBookmark.
         * @member {petajournal.ICreateBookmarkResponse|null|undefined} createBookmark
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.createBookmark = null;

        /**
         * ApiResponse deleteBookmark.
         * @member {petajournal.IDeleteBookmarkResponse|null|undefined} deleteBookmark
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.deleteBookmark = null;

        /**
         * ApiResponse deleteBookmarkGroup.
         * @member {petajournal.IDeleteBookmarkGroupResponse|null|undefined} deleteBookmarkGroup
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.deleteBookmarkGroup = null;

        /**
         * ApiResponse moveBookmark.
         * @member {petajournal.IMoveBookmarkResponse|null|undefined} moveBookmark
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.moveBookmark = null;

        /**
         * ApiResponse moveBookmarkGroup.
         * @member {petajournal.IMoveBookmarkGroupResponse|null|undefined} moveBookmarkGroup
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.moveBookmarkGroup = null;

        /**
         * ApiResponse updateBookmarkGroup.
         * @member {petajournal.IUpdateBookmarkGroupResponse|null|undefined} updateBookmarkGroup
         * @memberof petajournal.ApiResponse
         * @instance
         */
        ApiResponse.prototype.updateBookmarkGroup = null;

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
         * @member {"getPageById"|"getOrCreateDoc"|"deletePage"|"updatePage"|"createBlock"|"updateBlock"|"moveBlocks"|"deleteBlock"|"updateBlockStatus"|"searchPageNames"|"searchFullText"|"getRecentPages"|"getBacklinks"|"getMostRecentJournalBefore"|"getOrCreateJournal"|"getBlocksByStatus"|"getConfig"|"updateConfig"|"exportTreeMarkdown"|"importTreeMarkdown"|"getBookmarks"|"createBookmarkGroup"|"createBookmark"|"deleteBookmark"|"deleteBookmarkGroup"|"moveBookmark"|"moveBookmarkGroup"|"updateBookmarkGroup"|"serverEvent"|undefined} payload
         * @memberof petajournal.ApiResponse
         * @instance
         */
        Object.defineProperty(ApiResponse.prototype, "payload", {
            get: $util.oneOfGetter($oneOfFields = ["getPageById", "getOrCreateDoc", "deletePage", "updatePage", "createBlock", "updateBlock", "moveBlocks", "deleteBlock", "updateBlockStatus", "searchPageNames", "searchFullText", "getRecentPages", "getBacklinks", "getMostRecentJournalBefore", "getOrCreateJournal", "getBlocksByStatus", "getConfig", "updateConfig", "exportTreeMarkdown", "importTreeMarkdown", "getBookmarks", "createBookmarkGroup", "createBookmark", "deleteBookmark", "deleteBookmarkGroup", "moveBookmark", "moveBookmarkGroup", "updateBookmarkGroup", "serverEvent"]),
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
            if (message.getPageById != null && Object.hasOwnProperty.call(message, "getPageById"))
                $root.petajournal.GetPageByIdResponse.encode(message.getPageById, writer.uint32(/* id 10, wireType 2 =*/82).fork()).ldelim();
            if (message.getOrCreateDoc != null && Object.hasOwnProperty.call(message, "getOrCreateDoc"))
                $root.petajournal.GetOrCreateDocResponse.encode(message.getOrCreateDoc, writer.uint32(/* id 12, wireType 2 =*/98).fork()).ldelim();
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
            if (message.getOrCreateJournal != null && Object.hasOwnProperty.call(message, "getOrCreateJournal"))
                $root.petajournal.GetOrCreateJournalResponse.encode(message.getOrCreateJournal, writer.uint32(/* id 44, wireType 2 =*/354).fork()).ldelim();
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
            if (message.getBookmarks != null && Object.hasOwnProperty.call(message, "getBookmarks"))
                $root.petajournal.GetBookmarksResponse.encode(message.getBookmarks, writer.uint32(/* id 80, wireType 2 =*/642).fork()).ldelim();
            if (message.createBookmarkGroup != null && Object.hasOwnProperty.call(message, "createBookmarkGroup"))
                $root.petajournal.CreateBookmarkGroupResponse.encode(message.createBookmarkGroup, writer.uint32(/* id 81, wireType 2 =*/650).fork()).ldelim();
            if (message.createBookmark != null && Object.hasOwnProperty.call(message, "createBookmark"))
                $root.petajournal.CreateBookmarkResponse.encode(message.createBookmark, writer.uint32(/* id 82, wireType 2 =*/658).fork()).ldelim();
            if (message.deleteBookmark != null && Object.hasOwnProperty.call(message, "deleteBookmark"))
                $root.petajournal.DeleteBookmarkResponse.encode(message.deleteBookmark, writer.uint32(/* id 83, wireType 2 =*/666).fork()).ldelim();
            if (message.deleteBookmarkGroup != null && Object.hasOwnProperty.call(message, "deleteBookmarkGroup"))
                $root.petajournal.DeleteBookmarkGroupResponse.encode(message.deleteBookmarkGroup, writer.uint32(/* id 84, wireType 2 =*/674).fork()).ldelim();
            if (message.moveBookmark != null && Object.hasOwnProperty.call(message, "moveBookmark"))
                $root.petajournal.MoveBookmarkResponse.encode(message.moveBookmark, writer.uint32(/* id 85, wireType 2 =*/682).fork()).ldelim();
            if (message.moveBookmarkGroup != null && Object.hasOwnProperty.call(message, "moveBookmarkGroup"))
                $root.petajournal.MoveBookmarkGroupResponse.encode(message.moveBookmarkGroup, writer.uint32(/* id 86, wireType 2 =*/690).fork()).ldelim();
            if (message.updateBookmarkGroup != null && Object.hasOwnProperty.call(message, "updateBookmarkGroup"))
                $root.petajournal.UpdateBookmarkGroupResponse.encode(message.updateBookmarkGroup, writer.uint32(/* id 87, wireType 2 =*/698).fork()).ldelim();
            if (message.serverEvent != null && Object.hasOwnProperty.call(message, "serverEvent"))
                $root.petajournal.ServerEvent.encode(message.serverEvent, writer.uint32(/* id 100, wireType 2 =*/802).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified ApiResponse message, length delimited. Does not implicitly {@link petajournal.ApiResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ApiResponse
         * @static
         * @param {petajournal.IApiResponse} message ApiResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ApiResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                        message.getPageById = $root.petajournal.GetPageByIdResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 12: {
                        message.getOrCreateDoc = $root.petajournal.GetOrCreateDocResponse.decode(reader, reader.uint32());
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
                        message.getOrCreateJournal = $root.petajournal.GetOrCreateJournalResponse.decode(reader, reader.uint32());
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
                case 80: {
                        message.getBookmarks = $root.petajournal.GetBookmarksResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 81: {
                        message.createBookmarkGroup = $root.petajournal.CreateBookmarkGroupResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 82: {
                        message.createBookmark = $root.petajournal.CreateBookmarkResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 83: {
                        message.deleteBookmark = $root.petajournal.DeleteBookmarkResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 84: {
                        message.deleteBookmarkGroup = $root.petajournal.DeleteBookmarkGroupResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 85: {
                        message.moveBookmark = $root.petajournal.MoveBookmarkResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 86: {
                        message.moveBookmarkGroup = $root.petajournal.MoveBookmarkGroupResponse.decode(reader, reader.uint32());
                        break;
                    }
                case 87: {
                        message.updateBookmarkGroup = $root.petajournal.UpdateBookmarkGroupResponse.decode(reader, reader.uint32());
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
         * Decodes an ApiResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ApiResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ApiResponse} ApiResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ApiResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an ApiResponse message.
         * @function verify
         * @memberof petajournal.ApiResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ApiResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            let properties = {};
            if (message.requestId != null && message.hasOwnProperty("requestId"))
                if (!$util.isString(message.requestId))
                    return "requestId: string expected";
            if (message.success != null && message.hasOwnProperty("success"))
                if (typeof message.success !== "boolean")
                    return "success: boolean expected";
            if (message.errorMessage != null && message.hasOwnProperty("errorMessage"))
                if (!$util.isString(message.errorMessage))
                    return "errorMessage: string expected";
            if (message.getPageById != null && message.hasOwnProperty("getPageById")) {
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetPageByIdResponse.verify(message.getPageById);
                    if (error)
                        return "getPageById." + error;
                }
            }
            if (message.getOrCreateDoc != null && message.hasOwnProperty("getOrCreateDoc")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetOrCreateDocResponse.verify(message.getOrCreateDoc);
                    if (error)
                        return "getOrCreateDoc." + error;
                }
            }
            if (message.deletePage != null && message.hasOwnProperty("deletePage")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeletePageResponse.verify(message.deletePage);
                    if (error)
                        return "deletePage." + error;
                }
            }
            if (message.updatePage != null && message.hasOwnProperty("updatePage")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdatePageResponse.verify(message.updatePage);
                    if (error)
                        return "updatePage." + error;
                }
            }
            if (message.createBlock != null && message.hasOwnProperty("createBlock")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.CreateBlockResponse.verify(message.createBlock);
                    if (error)
                        return "createBlock." + error;
                }
            }
            if (message.updateBlock != null && message.hasOwnProperty("updateBlock")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateBlockResponse.verify(message.updateBlock);
                    if (error)
                        return "updateBlock." + error;
                }
            }
            if (message.moveBlocks != null && message.hasOwnProperty("moveBlocks")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.MoveBlocksResponse.verify(message.moveBlocks);
                    if (error)
                        return "moveBlocks." + error;
                }
            }
            if (message.deleteBlock != null && message.hasOwnProperty("deleteBlock")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeleteBlockResponse.verify(message.deleteBlock);
                    if (error)
                        return "deleteBlock." + error;
                }
            }
            if (message.updateBlockStatus != null && message.hasOwnProperty("updateBlockStatus")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateBlockStatusResponse.verify(message.updateBlockStatus);
                    if (error)
                        return "updateBlockStatus." + error;
                }
            }
            if (message.searchPageNames != null && message.hasOwnProperty("searchPageNames")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.SearchPageNamesResponse.verify(message.searchPageNames);
                    if (error)
                        return "searchPageNames." + error;
                }
            }
            if (message.searchFullText != null && message.hasOwnProperty("searchFullText")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.SearchFullTextResponse.verify(message.searchFullText);
                    if (error)
                        return "searchFullText." + error;
                }
            }
            if (message.getRecentPages != null && message.hasOwnProperty("getRecentPages")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetRecentPagesResponse.verify(message.getRecentPages);
                    if (error)
                        return "getRecentPages." + error;
                }
            }
            if (message.getBacklinks != null && message.hasOwnProperty("getBacklinks")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetBacklinksResponse.verify(message.getBacklinks);
                    if (error)
                        return "getBacklinks." + error;
                }
            }
            if (message.getMostRecentJournalBefore != null && message.hasOwnProperty("getMostRecentJournalBefore")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetMostRecentJournalBeforeResponse.verify(message.getMostRecentJournalBefore);
                    if (error)
                        return "getMostRecentJournalBefore." + error;
                }
            }
            if (message.getOrCreateJournal != null && message.hasOwnProperty("getOrCreateJournal")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetOrCreateJournalResponse.verify(message.getOrCreateJournal);
                    if (error)
                        return "getOrCreateJournal." + error;
                }
            }
            if (message.getBlocksByStatus != null && message.hasOwnProperty("getBlocksByStatus")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetBlocksByStatusResponse.verify(message.getBlocksByStatus);
                    if (error)
                        return "getBlocksByStatus." + error;
                }
            }
            if (message.getConfig != null && message.hasOwnProperty("getConfig")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetConfigResponse.verify(message.getConfig);
                    if (error)
                        return "getConfig." + error;
                }
            }
            if (message.updateConfig != null && message.hasOwnProperty("updateConfig")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateConfigResponse.verify(message.updateConfig);
                    if (error)
                        return "updateConfig." + error;
                }
            }
            if (message.exportTreeMarkdown != null && message.hasOwnProperty("exportTreeMarkdown")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.ExportTreeMarkdownResponse.verify(message.exportTreeMarkdown);
                    if (error)
                        return "exportTreeMarkdown." + error;
                }
            }
            if (message.importTreeMarkdown != null && message.hasOwnProperty("importTreeMarkdown")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.ImportTreeMarkdownResponse.verify(message.importTreeMarkdown);
                    if (error)
                        return "importTreeMarkdown." + error;
                }
            }
            if (message.getBookmarks != null && message.hasOwnProperty("getBookmarks")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.GetBookmarksResponse.verify(message.getBookmarks);
                    if (error)
                        return "getBookmarks." + error;
                }
            }
            if (message.createBookmarkGroup != null && message.hasOwnProperty("createBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.CreateBookmarkGroupResponse.verify(message.createBookmarkGroup);
                    if (error)
                        return "createBookmarkGroup." + error;
                }
            }
            if (message.createBookmark != null && message.hasOwnProperty("createBookmark")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.CreateBookmarkResponse.verify(message.createBookmark);
                    if (error)
                        return "createBookmark." + error;
                }
            }
            if (message.deleteBookmark != null && message.hasOwnProperty("deleteBookmark")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeleteBookmarkResponse.verify(message.deleteBookmark);
                    if (error)
                        return "deleteBookmark." + error;
                }
            }
            if (message.deleteBookmarkGroup != null && message.hasOwnProperty("deleteBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.DeleteBookmarkGroupResponse.verify(message.deleteBookmarkGroup);
                    if (error)
                        return "deleteBookmarkGroup." + error;
                }
            }
            if (message.moveBookmark != null && message.hasOwnProperty("moveBookmark")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.MoveBookmarkResponse.verify(message.moveBookmark);
                    if (error)
                        return "moveBookmark." + error;
                }
            }
            if (message.moveBookmarkGroup != null && message.hasOwnProperty("moveBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.MoveBookmarkGroupResponse.verify(message.moveBookmarkGroup);
                    if (error)
                        return "moveBookmarkGroup." + error;
                }
            }
            if (message.updateBookmarkGroup != null && message.hasOwnProperty("updateBookmarkGroup")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.UpdateBookmarkGroupResponse.verify(message.updateBookmarkGroup);
                    if (error)
                        return "updateBookmarkGroup." + error;
                }
            }
            if (message.serverEvent != null && message.hasOwnProperty("serverEvent")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.petajournal.ServerEvent.verify(message.serverEvent);
                    if (error)
                        return "serverEvent." + error;
                }
            }
            return null;
        };

        /**
         * Creates an ApiResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ApiResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ApiResponse} ApiResponse
         */
        ApiResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ApiResponse)
                return object;
            let message = new $root.petajournal.ApiResponse();
            if (object.requestId != null)
                message.requestId = String(object.requestId);
            if (object.success != null)
                message.success = Boolean(object.success);
            if (object.errorMessage != null)
                message.errorMessage = String(object.errorMessage);
            if (object.getPageById != null) {
                if (typeof object.getPageById !== "object")
                    throw TypeError(".petajournal.ApiResponse.getPageById: object expected");
                message.getPageById = $root.petajournal.GetPageByIdResponse.fromObject(object.getPageById);
            }
            if (object.getOrCreateDoc != null) {
                if (typeof object.getOrCreateDoc !== "object")
                    throw TypeError(".petajournal.ApiResponse.getOrCreateDoc: object expected");
                message.getOrCreateDoc = $root.petajournal.GetOrCreateDocResponse.fromObject(object.getOrCreateDoc);
            }
            if (object.deletePage != null) {
                if (typeof object.deletePage !== "object")
                    throw TypeError(".petajournal.ApiResponse.deletePage: object expected");
                message.deletePage = $root.petajournal.DeletePageResponse.fromObject(object.deletePage);
            }
            if (object.updatePage != null) {
                if (typeof object.updatePage !== "object")
                    throw TypeError(".petajournal.ApiResponse.updatePage: object expected");
                message.updatePage = $root.petajournal.UpdatePageResponse.fromObject(object.updatePage);
            }
            if (object.createBlock != null) {
                if (typeof object.createBlock !== "object")
                    throw TypeError(".petajournal.ApiResponse.createBlock: object expected");
                message.createBlock = $root.petajournal.CreateBlockResponse.fromObject(object.createBlock);
            }
            if (object.updateBlock != null) {
                if (typeof object.updateBlock !== "object")
                    throw TypeError(".petajournal.ApiResponse.updateBlock: object expected");
                message.updateBlock = $root.petajournal.UpdateBlockResponse.fromObject(object.updateBlock);
            }
            if (object.moveBlocks != null) {
                if (typeof object.moveBlocks !== "object")
                    throw TypeError(".petajournal.ApiResponse.moveBlocks: object expected");
                message.moveBlocks = $root.petajournal.MoveBlocksResponse.fromObject(object.moveBlocks);
            }
            if (object.deleteBlock != null) {
                if (typeof object.deleteBlock !== "object")
                    throw TypeError(".petajournal.ApiResponse.deleteBlock: object expected");
                message.deleteBlock = $root.petajournal.DeleteBlockResponse.fromObject(object.deleteBlock);
            }
            if (object.updateBlockStatus != null) {
                if (typeof object.updateBlockStatus !== "object")
                    throw TypeError(".petajournal.ApiResponse.updateBlockStatus: object expected");
                message.updateBlockStatus = $root.petajournal.UpdateBlockStatusResponse.fromObject(object.updateBlockStatus);
            }
            if (object.searchPageNames != null) {
                if (typeof object.searchPageNames !== "object")
                    throw TypeError(".petajournal.ApiResponse.searchPageNames: object expected");
                message.searchPageNames = $root.petajournal.SearchPageNamesResponse.fromObject(object.searchPageNames);
            }
            if (object.searchFullText != null) {
                if (typeof object.searchFullText !== "object")
                    throw TypeError(".petajournal.ApiResponse.searchFullText: object expected");
                message.searchFullText = $root.petajournal.SearchFullTextResponse.fromObject(object.searchFullText);
            }
            if (object.getRecentPages != null) {
                if (typeof object.getRecentPages !== "object")
                    throw TypeError(".petajournal.ApiResponse.getRecentPages: object expected");
                message.getRecentPages = $root.petajournal.GetRecentPagesResponse.fromObject(object.getRecentPages);
            }
            if (object.getBacklinks != null) {
                if (typeof object.getBacklinks !== "object")
                    throw TypeError(".petajournal.ApiResponse.getBacklinks: object expected");
                message.getBacklinks = $root.petajournal.GetBacklinksResponse.fromObject(object.getBacklinks);
            }
            if (object.getMostRecentJournalBefore != null) {
                if (typeof object.getMostRecentJournalBefore !== "object")
                    throw TypeError(".petajournal.ApiResponse.getMostRecentJournalBefore: object expected");
                message.getMostRecentJournalBefore = $root.petajournal.GetMostRecentJournalBeforeResponse.fromObject(object.getMostRecentJournalBefore);
            }
            if (object.getOrCreateJournal != null) {
                if (typeof object.getOrCreateJournal !== "object")
                    throw TypeError(".petajournal.ApiResponse.getOrCreateJournal: object expected");
                message.getOrCreateJournal = $root.petajournal.GetOrCreateJournalResponse.fromObject(object.getOrCreateJournal);
            }
            if (object.getBlocksByStatus != null) {
                if (typeof object.getBlocksByStatus !== "object")
                    throw TypeError(".petajournal.ApiResponse.getBlocksByStatus: object expected");
                message.getBlocksByStatus = $root.petajournal.GetBlocksByStatusResponse.fromObject(object.getBlocksByStatus);
            }
            if (object.getConfig != null) {
                if (typeof object.getConfig !== "object")
                    throw TypeError(".petajournal.ApiResponse.getConfig: object expected");
                message.getConfig = $root.petajournal.GetConfigResponse.fromObject(object.getConfig);
            }
            if (object.updateConfig != null) {
                if (typeof object.updateConfig !== "object")
                    throw TypeError(".petajournal.ApiResponse.updateConfig: object expected");
                message.updateConfig = $root.petajournal.UpdateConfigResponse.fromObject(object.updateConfig);
            }
            if (object.exportTreeMarkdown != null) {
                if (typeof object.exportTreeMarkdown !== "object")
                    throw TypeError(".petajournal.ApiResponse.exportTreeMarkdown: object expected");
                message.exportTreeMarkdown = $root.petajournal.ExportTreeMarkdownResponse.fromObject(object.exportTreeMarkdown);
            }
            if (object.importTreeMarkdown != null) {
                if (typeof object.importTreeMarkdown !== "object")
                    throw TypeError(".petajournal.ApiResponse.importTreeMarkdown: object expected");
                message.importTreeMarkdown = $root.petajournal.ImportTreeMarkdownResponse.fromObject(object.importTreeMarkdown);
            }
            if (object.getBookmarks != null) {
                if (typeof object.getBookmarks !== "object")
                    throw TypeError(".petajournal.ApiResponse.getBookmarks: object expected");
                message.getBookmarks = $root.petajournal.GetBookmarksResponse.fromObject(object.getBookmarks);
            }
            if (object.createBookmarkGroup != null) {
                if (typeof object.createBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiResponse.createBookmarkGroup: object expected");
                message.createBookmarkGroup = $root.petajournal.CreateBookmarkGroupResponse.fromObject(object.createBookmarkGroup);
            }
            if (object.createBookmark != null) {
                if (typeof object.createBookmark !== "object")
                    throw TypeError(".petajournal.ApiResponse.createBookmark: object expected");
                message.createBookmark = $root.petajournal.CreateBookmarkResponse.fromObject(object.createBookmark);
            }
            if (object.deleteBookmark != null) {
                if (typeof object.deleteBookmark !== "object")
                    throw TypeError(".petajournal.ApiResponse.deleteBookmark: object expected");
                message.deleteBookmark = $root.petajournal.DeleteBookmarkResponse.fromObject(object.deleteBookmark);
            }
            if (object.deleteBookmarkGroup != null) {
                if (typeof object.deleteBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiResponse.deleteBookmarkGroup: object expected");
                message.deleteBookmarkGroup = $root.petajournal.DeleteBookmarkGroupResponse.fromObject(object.deleteBookmarkGroup);
            }
            if (object.moveBookmark != null) {
                if (typeof object.moveBookmark !== "object")
                    throw TypeError(".petajournal.ApiResponse.moveBookmark: object expected");
                message.moveBookmark = $root.petajournal.MoveBookmarkResponse.fromObject(object.moveBookmark);
            }
            if (object.moveBookmarkGroup != null) {
                if (typeof object.moveBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiResponse.moveBookmarkGroup: object expected");
                message.moveBookmarkGroup = $root.petajournal.MoveBookmarkGroupResponse.fromObject(object.moveBookmarkGroup);
            }
            if (object.updateBookmarkGroup != null) {
                if (typeof object.updateBookmarkGroup !== "object")
                    throw TypeError(".petajournal.ApiResponse.updateBookmarkGroup: object expected");
                message.updateBookmarkGroup = $root.petajournal.UpdateBookmarkGroupResponse.fromObject(object.updateBookmarkGroup);
            }
            if (object.serverEvent != null) {
                if (typeof object.serverEvent !== "object")
                    throw TypeError(".petajournal.ApiResponse.serverEvent: object expected");
                message.serverEvent = $root.petajournal.ServerEvent.fromObject(object.serverEvent);
            }
            return message;
        };

        /**
         * Creates a plain object from an ApiResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ApiResponse
         * @static
         * @param {petajournal.ApiResponse} message ApiResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ApiResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.requestId = "";
                object.success = false;
                object.errorMessage = "";
            }
            if (message.requestId != null && message.hasOwnProperty("requestId"))
                object.requestId = message.requestId;
            if (message.success != null && message.hasOwnProperty("success"))
                object.success = message.success;
            if (message.errorMessage != null && message.hasOwnProperty("errorMessage"))
                object.errorMessage = message.errorMessage;
            if (message.getPageById != null && message.hasOwnProperty("getPageById")) {
                object.getPageById = $root.petajournal.GetPageByIdResponse.toObject(message.getPageById, options);
                if (options.oneofs)
                    object.payload = "getPageById";
            }
            if (message.getOrCreateDoc != null && message.hasOwnProperty("getOrCreateDoc")) {
                object.getOrCreateDoc = $root.petajournal.GetOrCreateDocResponse.toObject(message.getOrCreateDoc, options);
                if (options.oneofs)
                    object.payload = "getOrCreateDoc";
            }
            if (message.deletePage != null && message.hasOwnProperty("deletePage")) {
                object.deletePage = $root.petajournal.DeletePageResponse.toObject(message.deletePage, options);
                if (options.oneofs)
                    object.payload = "deletePage";
            }
            if (message.updatePage != null && message.hasOwnProperty("updatePage")) {
                object.updatePage = $root.petajournal.UpdatePageResponse.toObject(message.updatePage, options);
                if (options.oneofs)
                    object.payload = "updatePage";
            }
            if (message.createBlock != null && message.hasOwnProperty("createBlock")) {
                object.createBlock = $root.petajournal.CreateBlockResponse.toObject(message.createBlock, options);
                if (options.oneofs)
                    object.payload = "createBlock";
            }
            if (message.updateBlock != null && message.hasOwnProperty("updateBlock")) {
                object.updateBlock = $root.petajournal.UpdateBlockResponse.toObject(message.updateBlock, options);
                if (options.oneofs)
                    object.payload = "updateBlock";
            }
            if (message.moveBlocks != null && message.hasOwnProperty("moveBlocks")) {
                object.moveBlocks = $root.petajournal.MoveBlocksResponse.toObject(message.moveBlocks, options);
                if (options.oneofs)
                    object.payload = "moveBlocks";
            }
            if (message.deleteBlock != null && message.hasOwnProperty("deleteBlock")) {
                object.deleteBlock = $root.petajournal.DeleteBlockResponse.toObject(message.deleteBlock, options);
                if (options.oneofs)
                    object.payload = "deleteBlock";
            }
            if (message.updateBlockStatus != null && message.hasOwnProperty("updateBlockStatus")) {
                object.updateBlockStatus = $root.petajournal.UpdateBlockStatusResponse.toObject(message.updateBlockStatus, options);
                if (options.oneofs)
                    object.payload = "updateBlockStatus";
            }
            if (message.searchPageNames != null && message.hasOwnProperty("searchPageNames")) {
                object.searchPageNames = $root.petajournal.SearchPageNamesResponse.toObject(message.searchPageNames, options);
                if (options.oneofs)
                    object.payload = "searchPageNames";
            }
            if (message.searchFullText != null && message.hasOwnProperty("searchFullText")) {
                object.searchFullText = $root.petajournal.SearchFullTextResponse.toObject(message.searchFullText, options);
                if (options.oneofs)
                    object.payload = "searchFullText";
            }
            if (message.getRecentPages != null && message.hasOwnProperty("getRecentPages")) {
                object.getRecentPages = $root.petajournal.GetRecentPagesResponse.toObject(message.getRecentPages, options);
                if (options.oneofs)
                    object.payload = "getRecentPages";
            }
            if (message.getBacklinks != null && message.hasOwnProperty("getBacklinks")) {
                object.getBacklinks = $root.petajournal.GetBacklinksResponse.toObject(message.getBacklinks, options);
                if (options.oneofs)
                    object.payload = "getBacklinks";
            }
            if (message.getMostRecentJournalBefore != null && message.hasOwnProperty("getMostRecentJournalBefore")) {
                object.getMostRecentJournalBefore = $root.petajournal.GetMostRecentJournalBeforeResponse.toObject(message.getMostRecentJournalBefore, options);
                if (options.oneofs)
                    object.payload = "getMostRecentJournalBefore";
            }
            if (message.getOrCreateJournal != null && message.hasOwnProperty("getOrCreateJournal")) {
                object.getOrCreateJournal = $root.petajournal.GetOrCreateJournalResponse.toObject(message.getOrCreateJournal, options);
                if (options.oneofs)
                    object.payload = "getOrCreateJournal";
            }
            if (message.getBlocksByStatus != null && message.hasOwnProperty("getBlocksByStatus")) {
                object.getBlocksByStatus = $root.petajournal.GetBlocksByStatusResponse.toObject(message.getBlocksByStatus, options);
                if (options.oneofs)
                    object.payload = "getBlocksByStatus";
            }
            if (message.getConfig != null && message.hasOwnProperty("getConfig")) {
                object.getConfig = $root.petajournal.GetConfigResponse.toObject(message.getConfig, options);
                if (options.oneofs)
                    object.payload = "getConfig";
            }
            if (message.updateConfig != null && message.hasOwnProperty("updateConfig")) {
                object.updateConfig = $root.petajournal.UpdateConfigResponse.toObject(message.updateConfig, options);
                if (options.oneofs)
                    object.payload = "updateConfig";
            }
            if (message.exportTreeMarkdown != null && message.hasOwnProperty("exportTreeMarkdown")) {
                object.exportTreeMarkdown = $root.petajournal.ExportTreeMarkdownResponse.toObject(message.exportTreeMarkdown, options);
                if (options.oneofs)
                    object.payload = "exportTreeMarkdown";
            }
            if (message.importTreeMarkdown != null && message.hasOwnProperty("importTreeMarkdown")) {
                object.importTreeMarkdown = $root.petajournal.ImportTreeMarkdownResponse.toObject(message.importTreeMarkdown, options);
                if (options.oneofs)
                    object.payload = "importTreeMarkdown";
            }
            if (message.getBookmarks != null && message.hasOwnProperty("getBookmarks")) {
                object.getBookmarks = $root.petajournal.GetBookmarksResponse.toObject(message.getBookmarks, options);
                if (options.oneofs)
                    object.payload = "getBookmarks";
            }
            if (message.createBookmarkGroup != null && message.hasOwnProperty("createBookmarkGroup")) {
                object.createBookmarkGroup = $root.petajournal.CreateBookmarkGroupResponse.toObject(message.createBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "createBookmarkGroup";
            }
            if (message.createBookmark != null && message.hasOwnProperty("createBookmark")) {
                object.createBookmark = $root.petajournal.CreateBookmarkResponse.toObject(message.createBookmark, options);
                if (options.oneofs)
                    object.payload = "createBookmark";
            }
            if (message.deleteBookmark != null && message.hasOwnProperty("deleteBookmark")) {
                object.deleteBookmark = $root.petajournal.DeleteBookmarkResponse.toObject(message.deleteBookmark, options);
                if (options.oneofs)
                    object.payload = "deleteBookmark";
            }
            if (message.deleteBookmarkGroup != null && message.hasOwnProperty("deleteBookmarkGroup")) {
                object.deleteBookmarkGroup = $root.petajournal.DeleteBookmarkGroupResponse.toObject(message.deleteBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "deleteBookmarkGroup";
            }
            if (message.moveBookmark != null && message.hasOwnProperty("moveBookmark")) {
                object.moveBookmark = $root.petajournal.MoveBookmarkResponse.toObject(message.moveBookmark, options);
                if (options.oneofs)
                    object.payload = "moveBookmark";
            }
            if (message.moveBookmarkGroup != null && message.hasOwnProperty("moveBookmarkGroup")) {
                object.moveBookmarkGroup = $root.petajournal.MoveBookmarkGroupResponse.toObject(message.moveBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "moveBookmarkGroup";
            }
            if (message.updateBookmarkGroup != null && message.hasOwnProperty("updateBookmarkGroup")) {
                object.updateBookmarkGroup = $root.petajournal.UpdateBookmarkGroupResponse.toObject(message.updateBookmarkGroup, options);
                if (options.oneofs)
                    object.payload = "updateBookmarkGroup";
            }
            if (message.serverEvent != null && message.hasOwnProperty("serverEvent")) {
                object.serverEvent = $root.petajournal.ServerEvent.toObject(message.serverEvent, options);
                if (options.oneofs)
                    object.payload = "serverEvent";
            }
            return object;
        };

        /**
         * Converts this ApiResponse to JSON.
         * @function toJSON
         * @memberof petajournal.ApiResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ApiResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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

    petajournal.GetPageByIdRequest = (function() {

        /**
         * Properties of a GetPageByIdRequest.
         * @memberof petajournal
         * @interface IGetPageByIdRequest
         * @property {string|null} [pageId] GetPageByIdRequest pageId
         */

        /**
         * Constructs a new GetPageByIdRequest.
         * @memberof petajournal
         * @classdesc Represents a GetPageByIdRequest.
         * @implements IGetPageByIdRequest
         * @constructor
         * @param {petajournal.IGetPageByIdRequest=} [properties] Properties to set
         */
        function GetPageByIdRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetPageByIdRequest pageId.
         * @member {string} pageId
         * @memberof petajournal.GetPageByIdRequest
         * @instance
         */
        GetPageByIdRequest.prototype.pageId = "";

        /**
         * Creates a new GetPageByIdRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {petajournal.IGetPageByIdRequest=} [properties] Properties to set
         * @returns {petajournal.GetPageByIdRequest} GetPageByIdRequest instance
         */
        GetPageByIdRequest.create = function create(properties) {
            return new GetPageByIdRequest(properties);
        };

        /**
         * Encodes the specified GetPageByIdRequest message. Does not implicitly {@link petajournal.GetPageByIdRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {petajournal.IGetPageByIdRequest} message GetPageByIdRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetPageByIdRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.pageId != null && Object.hasOwnProperty.call(message, "pageId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.pageId);
            return writer;
        };

        /**
         * Encodes the specified GetPageByIdRequest message, length delimited. Does not implicitly {@link petajournal.GetPageByIdRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {petajournal.IGetPageByIdRequest} message GetPageByIdRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetPageByIdRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetPageByIdRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetPageByIdRequest} GetPageByIdRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetPageByIdRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetPageByIdRequest();
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
         * Decodes a GetPageByIdRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetPageByIdRequest} GetPageByIdRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetPageByIdRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetPageByIdRequest message.
         * @function verify
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetPageByIdRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            return null;
        };

        /**
         * Creates a GetPageByIdRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetPageByIdRequest} GetPageByIdRequest
         */
        GetPageByIdRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetPageByIdRequest)
                return object;
            let message = new $root.petajournal.GetPageByIdRequest();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            return message;
        };

        /**
         * Creates a plain object from a GetPageByIdRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {petajournal.GetPageByIdRequest} message GetPageByIdRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetPageByIdRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.pageId = "";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            return object;
        };

        /**
         * Converts this GetPageByIdRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetPageByIdRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetPageByIdRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetPageByIdRequest
         * @function getTypeUrl
         * @memberof petajournal.GetPageByIdRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetPageByIdRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetPageByIdRequest";
        };

        return GetPageByIdRequest;
    })();

    petajournal.GetPageByIdResponse = (function() {

        /**
         * Properties of a GetPageByIdResponse.
         * @memberof petajournal
         * @interface IGetPageByIdResponse
         * @property {petajournal.IPage|null} [page] GetPageByIdResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetPageByIdResponse blocks
         */

        /**
         * Constructs a new GetPageByIdResponse.
         * @memberof petajournal
         * @classdesc Represents a GetPageByIdResponse.
         * @implements IGetPageByIdResponse
         * @constructor
         * @param {petajournal.IGetPageByIdResponse=} [properties] Properties to set
         */
        function GetPageByIdResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetPageByIdResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetPageByIdResponse
         * @instance
         */
        GetPageByIdResponse.prototype.page = null;

        /**
         * GetPageByIdResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetPageByIdResponse
         * @instance
         */
        GetPageByIdResponse.prototype.blocks = $util.emptyArray;

        /**
         * Creates a new GetPageByIdResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {petajournal.IGetPageByIdResponse=} [properties] Properties to set
         * @returns {petajournal.GetPageByIdResponse} GetPageByIdResponse instance
         */
        GetPageByIdResponse.create = function create(properties) {
            return new GetPageByIdResponse(properties);
        };

        /**
         * Encodes the specified GetPageByIdResponse message. Does not implicitly {@link petajournal.GetPageByIdResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {petajournal.IGetPageByIdResponse} message GetPageByIdResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetPageByIdResponse.encode = function encode(message, writer) {
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
         * Encodes the specified GetPageByIdResponse message, length delimited. Does not implicitly {@link petajournal.GetPageByIdResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {petajournal.IGetPageByIdResponse} message GetPageByIdResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetPageByIdResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetPageByIdResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetPageByIdResponse} GetPageByIdResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetPageByIdResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetPageByIdResponse();
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
         * Decodes a GetPageByIdResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetPageByIdResponse} GetPageByIdResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetPageByIdResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetPageByIdResponse message.
         * @function verify
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetPageByIdResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.page != null && message.hasOwnProperty("page")) {
                let error = $root.petajournal.Page.verify(message.page);
                if (error)
                    return "page." + error;
            }
            if (message.blocks != null && message.hasOwnProperty("blocks")) {
                if (!Array.isArray(message.blocks))
                    return "blocks: array expected";
                for (let i = 0; i < message.blocks.length; ++i) {
                    let error = $root.petajournal.Block.verify(message.blocks[i]);
                    if (error)
                        return "blocks." + error;
                }
            }
            return null;
        };

        /**
         * Creates a GetPageByIdResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetPageByIdResponse} GetPageByIdResponse
         */
        GetPageByIdResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetPageByIdResponse)
                return object;
            let message = new $root.petajournal.GetPageByIdResponse();
            if (object.page != null) {
                if (typeof object.page !== "object")
                    throw TypeError(".petajournal.GetPageByIdResponse.page: object expected");
                message.page = $root.petajournal.Page.fromObject(object.page);
            }
            if (object.blocks) {
                if (!Array.isArray(object.blocks))
                    throw TypeError(".petajournal.GetPageByIdResponse.blocks: array expected");
                message.blocks = [];
                for (let i = 0; i < object.blocks.length; ++i) {
                    if (typeof object.blocks[i] !== "object")
                        throw TypeError(".petajournal.GetPageByIdResponse.blocks: object expected");
                    message.blocks[i] = $root.petajournal.Block.fromObject(object.blocks[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a GetPageByIdResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {petajournal.GetPageByIdResponse} message GetPageByIdResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetPageByIdResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blocks = [];
            if (options.defaults)
                object.page = null;
            if (message.page != null && message.hasOwnProperty("page"))
                object.page = $root.petajournal.Page.toObject(message.page, options);
            if (message.blocks && message.blocks.length) {
                object.blocks = [];
                for (let j = 0; j < message.blocks.length; ++j)
                    object.blocks[j] = $root.petajournal.Block.toObject(message.blocks[j], options);
            }
            return object;
        };

        /**
         * Converts this GetPageByIdResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetPageByIdResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetPageByIdResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetPageByIdResponse
         * @function getTypeUrl
         * @memberof petajournal.GetPageByIdResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetPageByIdResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetPageByIdResponse";
        };

        return GetPageByIdResponse;
    })();

    petajournal.GetOrCreateDocRequest = (function() {

        /**
         * Properties of a GetOrCreateDocRequest.
         * @memberof petajournal
         * @interface IGetOrCreateDocRequest
         * @property {string|null} [title] GetOrCreateDocRequest title
         * @property {string|null} [dirPath] GetOrCreateDocRequest dirPath
         * @property {Array.<string>|null} [tags] GetOrCreateDocRequest tags
         */

        /**
         * Constructs a new GetOrCreateDocRequest.
         * @memberof petajournal
         * @classdesc Represents a GetOrCreateDocRequest.
         * @implements IGetOrCreateDocRequest
         * @constructor
         * @param {petajournal.IGetOrCreateDocRequest=} [properties] Properties to set
         */
        function GetOrCreateDocRequest(properties) {
            this.tags = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetOrCreateDocRequest title.
         * @member {string} title
         * @memberof petajournal.GetOrCreateDocRequest
         * @instance
         */
        GetOrCreateDocRequest.prototype.title = "";

        /**
         * GetOrCreateDocRequest dirPath.
         * @member {string} dirPath
         * @memberof petajournal.GetOrCreateDocRequest
         * @instance
         */
        GetOrCreateDocRequest.prototype.dirPath = "";

        /**
         * GetOrCreateDocRequest tags.
         * @member {Array.<string>} tags
         * @memberof petajournal.GetOrCreateDocRequest
         * @instance
         */
        GetOrCreateDocRequest.prototype.tags = $util.emptyArray;

        /**
         * Creates a new GetOrCreateDocRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {petajournal.IGetOrCreateDocRequest=} [properties] Properties to set
         * @returns {petajournal.GetOrCreateDocRequest} GetOrCreateDocRequest instance
         */
        GetOrCreateDocRequest.create = function create(properties) {
            return new GetOrCreateDocRequest(properties);
        };

        /**
         * Encodes the specified GetOrCreateDocRequest message. Does not implicitly {@link petajournal.GetOrCreateDocRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {petajournal.IGetOrCreateDocRequest} message GetOrCreateDocRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateDocRequest.encode = function encode(message, writer) {
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
         * Encodes the specified GetOrCreateDocRequest message, length delimited. Does not implicitly {@link petajournal.GetOrCreateDocRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {petajournal.IGetOrCreateDocRequest} message GetOrCreateDocRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateDocRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetOrCreateDocRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetOrCreateDocRequest} GetOrCreateDocRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateDocRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetOrCreateDocRequest();
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
         * Decodes a GetOrCreateDocRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetOrCreateDocRequest} GetOrCreateDocRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateDocRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetOrCreateDocRequest message.
         * @function verify
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetOrCreateDocRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.title != null && message.hasOwnProperty("title"))
                if (!$util.isString(message.title))
                    return "title: string expected";
            if (message.dirPath != null && message.hasOwnProperty("dirPath"))
                if (!$util.isString(message.dirPath))
                    return "dirPath: string expected";
            if (message.tags != null && message.hasOwnProperty("tags")) {
                if (!Array.isArray(message.tags))
                    return "tags: array expected";
                for (let i = 0; i < message.tags.length; ++i)
                    if (!$util.isString(message.tags[i]))
                        return "tags: string[] expected";
            }
            return null;
        };

        /**
         * Creates a GetOrCreateDocRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetOrCreateDocRequest} GetOrCreateDocRequest
         */
        GetOrCreateDocRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetOrCreateDocRequest)
                return object;
            let message = new $root.petajournal.GetOrCreateDocRequest();
            if (object.title != null)
                message.title = String(object.title);
            if (object.dirPath != null)
                message.dirPath = String(object.dirPath);
            if (object.tags) {
                if (!Array.isArray(object.tags))
                    throw TypeError(".petajournal.GetOrCreateDocRequest.tags: array expected");
                message.tags = [];
                for (let i = 0; i < object.tags.length; ++i)
                    message.tags[i] = String(object.tags[i]);
            }
            return message;
        };

        /**
         * Creates a plain object from a GetOrCreateDocRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {petajournal.GetOrCreateDocRequest} message GetOrCreateDocRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetOrCreateDocRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.tags = [];
            if (options.defaults) {
                object.title = "";
                object.dirPath = "";
            }
            if (message.title != null && message.hasOwnProperty("title"))
                object.title = message.title;
            if (message.dirPath != null && message.hasOwnProperty("dirPath"))
                object.dirPath = message.dirPath;
            if (message.tags && message.tags.length) {
                object.tags = [];
                for (let j = 0; j < message.tags.length; ++j)
                    object.tags[j] = message.tags[j];
            }
            return object;
        };

        /**
         * Converts this GetOrCreateDocRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetOrCreateDocRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetOrCreateDocRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetOrCreateDocRequest
         * @function getTypeUrl
         * @memberof petajournal.GetOrCreateDocRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetOrCreateDocRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetOrCreateDocRequest";
        };

        return GetOrCreateDocRequest;
    })();

    petajournal.GetOrCreateDocResponse = (function() {

        /**
         * Properties of a GetOrCreateDocResponse.
         * @memberof petajournal
         * @interface IGetOrCreateDocResponse
         * @property {petajournal.IPage|null} [page] GetOrCreateDocResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetOrCreateDocResponse blocks
         * @property {boolean|null} [created] GetOrCreateDocResponse created
         */

        /**
         * Constructs a new GetOrCreateDocResponse.
         * @memberof petajournal
         * @classdesc Represents a GetOrCreateDocResponse.
         * @implements IGetOrCreateDocResponse
         * @constructor
         * @param {petajournal.IGetOrCreateDocResponse=} [properties] Properties to set
         */
        function GetOrCreateDocResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetOrCreateDocResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetOrCreateDocResponse
         * @instance
         */
        GetOrCreateDocResponse.prototype.page = null;

        /**
         * GetOrCreateDocResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetOrCreateDocResponse
         * @instance
         */
        GetOrCreateDocResponse.prototype.blocks = $util.emptyArray;

        /**
         * GetOrCreateDocResponse created.
         * @member {boolean} created
         * @memberof petajournal.GetOrCreateDocResponse
         * @instance
         */
        GetOrCreateDocResponse.prototype.created = false;

        /**
         * Creates a new GetOrCreateDocResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {petajournal.IGetOrCreateDocResponse=} [properties] Properties to set
         * @returns {petajournal.GetOrCreateDocResponse} GetOrCreateDocResponse instance
         */
        GetOrCreateDocResponse.create = function create(properties) {
            return new GetOrCreateDocResponse(properties);
        };

        /**
         * Encodes the specified GetOrCreateDocResponse message. Does not implicitly {@link petajournal.GetOrCreateDocResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {petajournal.IGetOrCreateDocResponse} message GetOrCreateDocResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateDocResponse.encode = function encode(message, writer) {
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
         * Encodes the specified GetOrCreateDocResponse message, length delimited. Does not implicitly {@link petajournal.GetOrCreateDocResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {petajournal.IGetOrCreateDocResponse} message GetOrCreateDocResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateDocResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetOrCreateDocResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetOrCreateDocResponse} GetOrCreateDocResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateDocResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetOrCreateDocResponse();
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
         * Decodes a GetOrCreateDocResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetOrCreateDocResponse} GetOrCreateDocResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateDocResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetOrCreateDocResponse message.
         * @function verify
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetOrCreateDocResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.page != null && message.hasOwnProperty("page")) {
                let error = $root.petajournal.Page.verify(message.page);
                if (error)
                    return "page." + error;
            }
            if (message.blocks != null && message.hasOwnProperty("blocks")) {
                if (!Array.isArray(message.blocks))
                    return "blocks: array expected";
                for (let i = 0; i < message.blocks.length; ++i) {
                    let error = $root.petajournal.Block.verify(message.blocks[i]);
                    if (error)
                        return "blocks." + error;
                }
            }
            if (message.created != null && message.hasOwnProperty("created"))
                if (typeof message.created !== "boolean")
                    return "created: boolean expected";
            return null;
        };

        /**
         * Creates a GetOrCreateDocResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetOrCreateDocResponse} GetOrCreateDocResponse
         */
        GetOrCreateDocResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetOrCreateDocResponse)
                return object;
            let message = new $root.petajournal.GetOrCreateDocResponse();
            if (object.page != null) {
                if (typeof object.page !== "object")
                    throw TypeError(".petajournal.GetOrCreateDocResponse.page: object expected");
                message.page = $root.petajournal.Page.fromObject(object.page);
            }
            if (object.blocks) {
                if (!Array.isArray(object.blocks))
                    throw TypeError(".petajournal.GetOrCreateDocResponse.blocks: array expected");
                message.blocks = [];
                for (let i = 0; i < object.blocks.length; ++i) {
                    if (typeof object.blocks[i] !== "object")
                        throw TypeError(".petajournal.GetOrCreateDocResponse.blocks: object expected");
                    message.blocks[i] = $root.petajournal.Block.fromObject(object.blocks[i]);
                }
            }
            if (object.created != null)
                message.created = Boolean(object.created);
            return message;
        };

        /**
         * Creates a plain object from a GetOrCreateDocResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {petajournal.GetOrCreateDocResponse} message GetOrCreateDocResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetOrCreateDocResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blocks = [];
            if (options.defaults) {
                object.page = null;
                object.created = false;
            }
            if (message.page != null && message.hasOwnProperty("page"))
                object.page = $root.petajournal.Page.toObject(message.page, options);
            if (message.blocks && message.blocks.length) {
                object.blocks = [];
                for (let j = 0; j < message.blocks.length; ++j)
                    object.blocks[j] = $root.petajournal.Block.toObject(message.blocks[j], options);
            }
            if (message.created != null && message.hasOwnProperty("created"))
                object.created = message.created;
            return object;
        };

        /**
         * Converts this GetOrCreateDocResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetOrCreateDocResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetOrCreateDocResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetOrCreateDocResponse
         * @function getTypeUrl
         * @memberof petajournal.GetOrCreateDocResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetOrCreateDocResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetOrCreateDocResponse";
        };

        return GetOrCreateDocResponse;
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
         * Encodes the specified DeletePageRequest message, length delimited. Does not implicitly {@link petajournal.DeletePageRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {petajournal.IDeletePageRequest} message DeletePageRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeletePageRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a DeletePageRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeletePageRequest} DeletePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeletePageRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeletePageRequest message.
         * @function verify
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeletePageRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            return null;
        };

        /**
         * Creates a DeletePageRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeletePageRequest} DeletePageRequest
         */
        DeletePageRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeletePageRequest)
                return object;
            let message = new $root.petajournal.DeletePageRequest();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            return message;
        };

        /**
         * Creates a plain object from a DeletePageRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeletePageRequest
         * @static
         * @param {petajournal.DeletePageRequest} message DeletePageRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeletePageRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.pageId = "";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            return object;
        };

        /**
         * Converts this DeletePageRequest to JSON.
         * @function toJSON
         * @memberof petajournal.DeletePageRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeletePageRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified DeletePageResponse message, length delimited. Does not implicitly {@link petajournal.DeletePageResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {petajournal.IDeletePageResponse} message DeletePageResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeletePageResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a DeletePageResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeletePageResponse} DeletePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeletePageResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeletePageResponse message.
         * @function verify
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeletePageResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            return null;
        };

        /**
         * Creates a DeletePageResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeletePageResponse} DeletePageResponse
         */
        DeletePageResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeletePageResponse)
                return object;
            return new $root.petajournal.DeletePageResponse();
        };

        /**
         * Creates a plain object from a DeletePageResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeletePageResponse
         * @static
         * @param {petajournal.DeletePageResponse} message DeletePageResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeletePageResponse.toObject = function toObject() {
            return {};
        };

        /**
         * Converts this DeletePageResponse to JSON.
         * @function toJSON
         * @memberof petajournal.DeletePageResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeletePageResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * @property {string|null} [dirPath] UpdatePageRequest dirPath
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
         * UpdatePageRequest dirPath.
         * @member {string|null|undefined} dirPath
         * @memberof petajournal.UpdatePageRequest
         * @instance
         */
        UpdatePageRequest.prototype.dirPath = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdatePageRequest.prototype, "_title", {
            get: $util.oneOfGetter($oneOfFields = ["title"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        // Virtual OneOf for proto3 optional field
        Object.defineProperty(UpdatePageRequest.prototype, "_dirPath", {
            get: $util.oneOfGetter($oneOfFields = ["dirPath"]),
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
            if (message.dirPath != null && Object.hasOwnProperty.call(message, "dirPath"))
                writer.uint32(/* id 5, wireType 2 =*/42).string(message.dirPath);
            return writer;
        };

        /**
         * Encodes the specified UpdatePageRequest message, length delimited. Does not implicitly {@link petajournal.UpdatePageRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {petajournal.IUpdatePageRequest} message UpdatePageRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdatePageRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                case 5: {
                        message.dirPath = reader.string();
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
         * Decodes an UpdatePageRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdatePageRequest} UpdatePageRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdatePageRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdatePageRequest message.
         * @function verify
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdatePageRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            let properties = {};
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            if (message.title != null && message.hasOwnProperty("title")) {
                properties._title = 1;
                if (!$util.isString(message.title))
                    return "title: string expected";
            }
            if (message.tags != null && message.hasOwnProperty("tags")) {
                if (!Array.isArray(message.tags))
                    return "tags: array expected";
                for (let i = 0; i < message.tags.length; ++i)
                    if (!$util.isString(message.tags[i]))
                        return "tags: string[] expected";
            }
            if (message.dirPath != null && message.hasOwnProperty("dirPath")) {
                properties._dirPath = 1;
                if (!$util.isString(message.dirPath))
                    return "dirPath: string expected";
            }
            return null;
        };

        /**
         * Creates an UpdatePageRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdatePageRequest} UpdatePageRequest
         */
        UpdatePageRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdatePageRequest)
                return object;
            let message = new $root.petajournal.UpdatePageRequest();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            if (object.title != null)
                message.title = String(object.title);
            if (object.tags) {
                if (!Array.isArray(object.tags))
                    throw TypeError(".petajournal.UpdatePageRequest.tags: array expected");
                message.tags = [];
                for (let i = 0; i < object.tags.length; ++i)
                    message.tags[i] = String(object.tags[i]);
            }
            if (object.dirPath != null)
                message.dirPath = String(object.dirPath);
            return message;
        };

        /**
         * Creates a plain object from an UpdatePageRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdatePageRequest
         * @static
         * @param {petajournal.UpdatePageRequest} message UpdatePageRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdatePageRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.tags = [];
            if (options.defaults)
                object.pageId = "";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            if (message.title != null && message.hasOwnProperty("title")) {
                object.title = message.title;
                if (options.oneofs)
                    object._title = "title";
            }
            if (message.tags && message.tags.length) {
                object.tags = [];
                for (let j = 0; j < message.tags.length; ++j)
                    object.tags[j] = message.tags[j];
            }
            if (message.dirPath != null && message.hasOwnProperty("dirPath")) {
                object.dirPath = message.dirPath;
                if (options.oneofs)
                    object._dirPath = "dirPath";
            }
            return object;
        };

        /**
         * Converts this UpdatePageRequest to JSON.
         * @function toJSON
         * @memberof petajournal.UpdatePageRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdatePageRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified UpdatePageResponse message, length delimited. Does not implicitly {@link petajournal.UpdatePageResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {petajournal.IUpdatePageResponse} message UpdatePageResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdatePageResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an UpdatePageResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdatePageResponse} UpdatePageResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdatePageResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdatePageResponse message.
         * @function verify
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdatePageResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.page != null && message.hasOwnProperty("page")) {
                let error = $root.petajournal.Page.verify(message.page);
                if (error)
                    return "page." + error;
            }
            return null;
        };

        /**
         * Creates an UpdatePageResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdatePageResponse} UpdatePageResponse
         */
        UpdatePageResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdatePageResponse)
                return object;
            let message = new $root.petajournal.UpdatePageResponse();
            if (object.page != null) {
                if (typeof object.page !== "object")
                    throw TypeError(".petajournal.UpdatePageResponse.page: object expected");
                message.page = $root.petajournal.Page.fromObject(object.page);
            }
            return message;
        };

        /**
         * Creates a plain object from an UpdatePageResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdatePageResponse
         * @static
         * @param {petajournal.UpdatePageResponse} message UpdatePageResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdatePageResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.page = null;
            if (message.page != null && message.hasOwnProperty("page"))
                object.page = $root.petajournal.Page.toObject(message.page, options);
            return object;
        };

        /**
         * Converts this UpdatePageResponse to JSON.
         * @function toJSON
         * @memberof petajournal.UpdatePageResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdatePageResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * @property {string|null} [id] CreateBlockRequest id
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
         * CreateBlockRequest id.
         * @member {string} id
         * @memberof petajournal.CreateBlockRequest
         * @instance
         */
        CreateBlockRequest.prototype.id = "";

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
            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                writer.uint32(/* id 5, wireType 2 =*/42).string(message.id);
            return writer;
        };

        /**
         * Encodes the specified CreateBlockRequest message, length delimited. Does not implicitly {@link petajournal.CreateBlockRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {petajournal.ICreateBlockRequest} message CreateBlockRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBlockRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                case 5: {
                        message.id = reader.string();
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
         * Decodes a CreateBlockRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.CreateBlockRequest} CreateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBlockRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a CreateBlockRequest message.
         * @function verify
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        CreateBlockRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            if (message.afterBlockId != null && message.hasOwnProperty("afterBlockId"))
                if (!$util.isString(message.afterBlockId))
                    return "afterBlockId: string expected";
            if (message.level != null && message.hasOwnProperty("level"))
                if (!$util.isInteger(message.level))
                    return "level: integer expected";
            if (message.body != null && message.hasOwnProperty("body"))
                if (!$util.isString(message.body))
                    return "body: string expected";
            if (message.id != null && message.hasOwnProperty("id"))
                if (!$util.isString(message.id))
                    return "id: string expected";
            return null;
        };

        /**
         * Creates a CreateBlockRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.CreateBlockRequest} CreateBlockRequest
         */
        CreateBlockRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.CreateBlockRequest)
                return object;
            let message = new $root.petajournal.CreateBlockRequest();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            if (object.afterBlockId != null)
                message.afterBlockId = String(object.afterBlockId);
            if (object.level != null)
                message.level = object.level | 0;
            if (object.body != null)
                message.body = String(object.body);
            if (object.id != null)
                message.id = String(object.id);
            return message;
        };

        /**
         * Creates a plain object from a CreateBlockRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.CreateBlockRequest
         * @static
         * @param {petajournal.CreateBlockRequest} message CreateBlockRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        CreateBlockRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.pageId = "";
                object.afterBlockId = "";
                object.level = 0;
                object.body = "";
                object.id = "";
            }
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            if (message.afterBlockId != null && message.hasOwnProperty("afterBlockId"))
                object.afterBlockId = message.afterBlockId;
            if (message.level != null && message.hasOwnProperty("level"))
                object.level = message.level;
            if (message.body != null && message.hasOwnProperty("body"))
                object.body = message.body;
            if (message.id != null && message.hasOwnProperty("id"))
                object.id = message.id;
            return object;
        };

        /**
         * Converts this CreateBlockRequest to JSON.
         * @function toJSON
         * @memberof petajournal.CreateBlockRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        CreateBlockRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified CreateBlockResponse message, length delimited. Does not implicitly {@link petajournal.CreateBlockResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {petajournal.ICreateBlockResponse} message CreateBlockResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBlockResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a CreateBlockResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.CreateBlockResponse} CreateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBlockResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a CreateBlockResponse message.
         * @function verify
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        CreateBlockResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.block != null && message.hasOwnProperty("block")) {
                let error = $root.petajournal.Block.verify(message.block);
                if (error)
                    return "block." + error;
            }
            return null;
        };

        /**
         * Creates a CreateBlockResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.CreateBlockResponse} CreateBlockResponse
         */
        CreateBlockResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.CreateBlockResponse)
                return object;
            let message = new $root.petajournal.CreateBlockResponse();
            if (object.block != null) {
                if (typeof object.block !== "object")
                    throw TypeError(".petajournal.CreateBlockResponse.block: object expected");
                message.block = $root.petajournal.Block.fromObject(object.block);
            }
            return message;
        };

        /**
         * Creates a plain object from a CreateBlockResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.CreateBlockResponse
         * @static
         * @param {petajournal.CreateBlockResponse} message CreateBlockResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        CreateBlockResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.block = null;
            if (message.block != null && message.hasOwnProperty("block"))
                object.block = $root.petajournal.Block.toObject(message.block, options);
            return object;
        };

        /**
         * Converts this CreateBlockResponse to JSON.
         * @function toJSON
         * @memberof petajournal.CreateBlockResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        CreateBlockResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
            return writer;
        };

        /**
         * Encodes the specified UpdateBlockRequest message, length delimited. Does not implicitly {@link petajournal.UpdateBlockRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {petajournal.IUpdateBlockRequest} message UpdateBlockRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Decodes an UpdateBlockRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateBlockRequest} UpdateBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateBlockRequest message.
         * @function verify
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateBlockRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            let properties = {};
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                if (!$util.isString(message.blockId))
                    return "blockId: string expected";
            if (message.body != null && message.hasOwnProperty("body")) {
                properties._body = 1;
                if (!$util.isString(message.body))
                    return "body: string expected";
            }
            if (message.level != null && message.hasOwnProperty("level")) {
                properties._level = 1;
                if (!$util.isInteger(message.level))
                    return "level: integer expected";
            }
            if (message.collapsed != null && message.hasOwnProperty("collapsed")) {
                properties._collapsed = 1;
                if (typeof message.collapsed !== "boolean")
                    return "collapsed: boolean expected";
            }
            if (message.status != null && message.hasOwnProperty("status")) {
                properties._status = 1;
                if (!$util.isString(message.status))
                    return "status: string expected";
            }
            return null;
        };

        /**
         * Creates an UpdateBlockRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateBlockRequest} UpdateBlockRequest
         */
        UpdateBlockRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateBlockRequest)
                return object;
            let message = new $root.petajournal.UpdateBlockRequest();
            if (object.blockId != null)
                message.blockId = String(object.blockId);
            if (object.body != null)
                message.body = String(object.body);
            if (object.level != null)
                message.level = object.level | 0;
            if (object.collapsed != null)
                message.collapsed = Boolean(object.collapsed);
            if (object.status != null)
                message.status = String(object.status);
            return message;
        };

        /**
         * Creates a plain object from an UpdateBlockRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateBlockRequest
         * @static
         * @param {petajournal.UpdateBlockRequest} message UpdateBlockRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateBlockRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.blockId = "";
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                object.blockId = message.blockId;
            if (message.body != null && message.hasOwnProperty("body")) {
                object.body = message.body;
                if (options.oneofs)
                    object._body = "body";
            }
            if (message.level != null && message.hasOwnProperty("level")) {
                object.level = message.level;
                if (options.oneofs)
                    object._level = "level";
            }
            if (message.collapsed != null && message.hasOwnProperty("collapsed")) {
                object.collapsed = message.collapsed;
                if (options.oneofs)
                    object._collapsed = "collapsed";
            }
            if (message.status != null && message.hasOwnProperty("status")) {
                object.status = message.status;
                if (options.oneofs)
                    object._status = "status";
            }
            return object;
        };

        /**
         * Converts this UpdateBlockRequest to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateBlockRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateBlockRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified UpdateBlockResponse message, length delimited. Does not implicitly {@link petajournal.UpdateBlockResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {petajournal.IUpdateBlockResponse} message UpdateBlockResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an UpdateBlockResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateBlockResponse} UpdateBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateBlockResponse message.
         * @function verify
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateBlockResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.block != null && message.hasOwnProperty("block")) {
                let error = $root.petajournal.Block.verify(message.block);
                if (error)
                    return "block." + error;
            }
            return null;
        };

        /**
         * Creates an UpdateBlockResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateBlockResponse} UpdateBlockResponse
         */
        UpdateBlockResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateBlockResponse)
                return object;
            let message = new $root.petajournal.UpdateBlockResponse();
            if (object.block != null) {
                if (typeof object.block !== "object")
                    throw TypeError(".petajournal.UpdateBlockResponse.block: object expected");
                message.block = $root.petajournal.Block.fromObject(object.block);
            }
            return message;
        };

        /**
         * Creates a plain object from an UpdateBlockResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateBlockResponse
         * @static
         * @param {petajournal.UpdateBlockResponse} message UpdateBlockResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateBlockResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.block = null;
            if (message.block != null && message.hasOwnProperty("block"))
                object.block = $root.petajournal.Block.toObject(message.block, options);
            return object;
        };

        /**
         * Converts this UpdateBlockResponse to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateBlockResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateBlockResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified MoveBlocksRequest message, length delimited. Does not implicitly {@link petajournal.MoveBlocksRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {petajournal.IMoveBlocksRequest} message MoveBlocksRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBlocksRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a MoveBlocksRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.MoveBlocksRequest} MoveBlocksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBlocksRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a MoveBlocksRequest message.
         * @function verify
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        MoveBlocksRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            let properties = {};
            if (message.blockIds != null && message.hasOwnProperty("blockIds")) {
                if (!Array.isArray(message.blockIds))
                    return "blockIds: array expected";
                for (let i = 0; i < message.blockIds.length; ++i)
                    if (!$util.isString(message.blockIds[i]))
                        return "blockIds: string[] expected";
            }
            if (message.targetPageId != null && message.hasOwnProperty("targetPageId"))
                if (!$util.isString(message.targetPageId))
                    return "targetPageId: string expected";
            if (message.afterBlockId != null && message.hasOwnProperty("afterBlockId"))
                if (!$util.isString(message.afterBlockId))
                    return "afterBlockId: string expected";
            if (message.levelDelta != null && message.hasOwnProperty("levelDelta")) {
                properties._levelDelta = 1;
                if (!$util.isInteger(message.levelDelta))
                    return "levelDelta: integer expected";
            }
            return null;
        };

        /**
         * Creates a MoveBlocksRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.MoveBlocksRequest} MoveBlocksRequest
         */
        MoveBlocksRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.MoveBlocksRequest)
                return object;
            let message = new $root.petajournal.MoveBlocksRequest();
            if (object.blockIds) {
                if (!Array.isArray(object.blockIds))
                    throw TypeError(".petajournal.MoveBlocksRequest.blockIds: array expected");
                message.blockIds = [];
                for (let i = 0; i < object.blockIds.length; ++i)
                    message.blockIds[i] = String(object.blockIds[i]);
            }
            if (object.targetPageId != null)
                message.targetPageId = String(object.targetPageId);
            if (object.afterBlockId != null)
                message.afterBlockId = String(object.afterBlockId);
            if (object.levelDelta != null)
                message.levelDelta = object.levelDelta | 0;
            return message;
        };

        /**
         * Creates a plain object from a MoveBlocksRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.MoveBlocksRequest
         * @static
         * @param {petajournal.MoveBlocksRequest} message MoveBlocksRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        MoveBlocksRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blockIds = [];
            if (options.defaults) {
                object.targetPageId = "";
                object.afterBlockId = "";
            }
            if (message.blockIds && message.blockIds.length) {
                object.blockIds = [];
                for (let j = 0; j < message.blockIds.length; ++j)
                    object.blockIds[j] = message.blockIds[j];
            }
            if (message.targetPageId != null && message.hasOwnProperty("targetPageId"))
                object.targetPageId = message.targetPageId;
            if (message.afterBlockId != null && message.hasOwnProperty("afterBlockId"))
                object.afterBlockId = message.afterBlockId;
            if (message.levelDelta != null && message.hasOwnProperty("levelDelta")) {
                object.levelDelta = message.levelDelta;
                if (options.oneofs)
                    object._levelDelta = "levelDelta";
            }
            return object;
        };

        /**
         * Converts this MoveBlocksRequest to JSON.
         * @function toJSON
         * @memberof petajournal.MoveBlocksRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        MoveBlocksRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified MoveBlocksResponse message, length delimited. Does not implicitly {@link petajournal.MoveBlocksResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {petajournal.IMoveBlocksResponse} message MoveBlocksResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBlocksResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a MoveBlocksResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.MoveBlocksResponse} MoveBlocksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBlocksResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a MoveBlocksResponse message.
         * @function verify
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        MoveBlocksResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.blocks != null && message.hasOwnProperty("blocks")) {
                if (!Array.isArray(message.blocks))
                    return "blocks: array expected";
                for (let i = 0; i < message.blocks.length; ++i) {
                    let error = $root.petajournal.Block.verify(message.blocks[i]);
                    if (error)
                        return "blocks." + error;
                }
            }
            return null;
        };

        /**
         * Creates a MoveBlocksResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.MoveBlocksResponse} MoveBlocksResponse
         */
        MoveBlocksResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.MoveBlocksResponse)
                return object;
            let message = new $root.petajournal.MoveBlocksResponse();
            if (object.blocks) {
                if (!Array.isArray(object.blocks))
                    throw TypeError(".petajournal.MoveBlocksResponse.blocks: array expected");
                message.blocks = [];
                for (let i = 0; i < object.blocks.length; ++i) {
                    if (typeof object.blocks[i] !== "object")
                        throw TypeError(".petajournal.MoveBlocksResponse.blocks: object expected");
                    message.blocks[i] = $root.petajournal.Block.fromObject(object.blocks[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a MoveBlocksResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.MoveBlocksResponse
         * @static
         * @param {petajournal.MoveBlocksResponse} message MoveBlocksResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        MoveBlocksResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blocks = [];
            if (message.blocks && message.blocks.length) {
                object.blocks = [];
                for (let j = 0; j < message.blocks.length; ++j)
                    object.blocks[j] = $root.petajournal.Block.toObject(message.blocks[j], options);
            }
            return object;
        };

        /**
         * Converts this MoveBlocksResponse to JSON.
         * @function toJSON
         * @memberof petajournal.MoveBlocksResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        MoveBlocksResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified DeleteBlockRequest message, length delimited. Does not implicitly {@link petajournal.DeleteBlockRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {petajournal.IDeleteBlockRequest} message DeleteBlockRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBlockRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a DeleteBlockRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeleteBlockRequest} DeleteBlockRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBlockRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeleteBlockRequest message.
         * @function verify
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeleteBlockRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                if (!$util.isString(message.blockId))
                    return "blockId: string expected";
            return null;
        };

        /**
         * Creates a DeleteBlockRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeleteBlockRequest} DeleteBlockRequest
         */
        DeleteBlockRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeleteBlockRequest)
                return object;
            let message = new $root.petajournal.DeleteBlockRequest();
            if (object.blockId != null)
                message.blockId = String(object.blockId);
            return message;
        };

        /**
         * Creates a plain object from a DeleteBlockRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeleteBlockRequest
         * @static
         * @param {petajournal.DeleteBlockRequest} message DeleteBlockRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeleteBlockRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.blockId = "";
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                object.blockId = message.blockId;
            return object;
        };

        /**
         * Converts this DeleteBlockRequest to JSON.
         * @function toJSON
         * @memberof petajournal.DeleteBlockRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeleteBlockRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified DeleteBlockResponse message, length delimited. Does not implicitly {@link petajournal.DeleteBlockResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {petajournal.IDeleteBlockResponse} message DeleteBlockResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBlockResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a DeleteBlockResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeleteBlockResponse} DeleteBlockResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBlockResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeleteBlockResponse message.
         * @function verify
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeleteBlockResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            return null;
        };

        /**
         * Creates a DeleteBlockResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeleteBlockResponse} DeleteBlockResponse
         */
        DeleteBlockResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeleteBlockResponse)
                return object;
            return new $root.petajournal.DeleteBlockResponse();
        };

        /**
         * Creates a plain object from a DeleteBlockResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeleteBlockResponse
         * @static
         * @param {petajournal.DeleteBlockResponse} message DeleteBlockResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeleteBlockResponse.toObject = function toObject() {
            return {};
        };

        /**
         * Converts this DeleteBlockResponse to JSON.
         * @function toJSON
         * @memberof petajournal.DeleteBlockResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeleteBlockResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified UpdateBlockStatusRequest message, length delimited. Does not implicitly {@link petajournal.UpdateBlockStatusRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {petajournal.IUpdateBlockStatusRequest} message UpdateBlockStatusRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockStatusRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an UpdateBlockStatusRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateBlockStatusRequest} UpdateBlockStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockStatusRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateBlockStatusRequest message.
         * @function verify
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateBlockStatusRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                if (!$util.isString(message.blockId))
                    return "blockId: string expected";
            if (message.status != null && message.hasOwnProperty("status"))
                if (!$util.isString(message.status))
                    return "status: string expected";
            return null;
        };

        /**
         * Creates an UpdateBlockStatusRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateBlockStatusRequest} UpdateBlockStatusRequest
         */
        UpdateBlockStatusRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateBlockStatusRequest)
                return object;
            let message = new $root.petajournal.UpdateBlockStatusRequest();
            if (object.blockId != null)
                message.blockId = String(object.blockId);
            if (object.status != null)
                message.status = String(object.status);
            return message;
        };

        /**
         * Creates a plain object from an UpdateBlockStatusRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateBlockStatusRequest
         * @static
         * @param {petajournal.UpdateBlockStatusRequest} message UpdateBlockStatusRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateBlockStatusRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.blockId = "";
                object.status = "";
            }
            if (message.blockId != null && message.hasOwnProperty("blockId"))
                object.blockId = message.blockId;
            if (message.status != null && message.hasOwnProperty("status"))
                object.status = message.status;
            return object;
        };

        /**
         * Converts this UpdateBlockStatusRequest to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateBlockStatusRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateBlockStatusRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified UpdateBlockStatusResponse message, length delimited. Does not implicitly {@link petajournal.UpdateBlockStatusResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {petajournal.IUpdateBlockStatusResponse} message UpdateBlockStatusResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBlockStatusResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an UpdateBlockStatusResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateBlockStatusResponse} UpdateBlockStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBlockStatusResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateBlockStatusResponse message.
         * @function verify
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateBlockStatusResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.block != null && message.hasOwnProperty("block")) {
                let error = $root.petajournal.Block.verify(message.block);
                if (error)
                    return "block." + error;
            }
            return null;
        };

        /**
         * Creates an UpdateBlockStatusResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateBlockStatusResponse} UpdateBlockStatusResponse
         */
        UpdateBlockStatusResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateBlockStatusResponse)
                return object;
            let message = new $root.petajournal.UpdateBlockStatusResponse();
            if (object.block != null) {
                if (typeof object.block !== "object")
                    throw TypeError(".petajournal.UpdateBlockStatusResponse.block: object expected");
                message.block = $root.petajournal.Block.fromObject(object.block);
            }
            return message;
        };

        /**
         * Creates a plain object from an UpdateBlockStatusResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateBlockStatusResponse
         * @static
         * @param {petajournal.UpdateBlockStatusResponse} message UpdateBlockStatusResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateBlockStatusResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.block = null;
            if (message.block != null && message.hasOwnProperty("block"))
                object.block = $root.petajournal.Block.toObject(message.block, options);
            return object;
        };

        /**
         * Converts this UpdateBlockStatusResponse to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateBlockStatusResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateBlockStatusResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchPageNamesRequest message, length delimited. Does not implicitly {@link petajournal.SearchPageNamesRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {petajournal.ISearchPageNamesRequest} message SearchPageNamesRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchPageNamesRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchPageNamesRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchPageNamesRequest} SearchPageNamesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchPageNamesRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchPageNamesRequest message.
         * @function verify
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchPageNamesRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.query != null && message.hasOwnProperty("query"))
                if (!$util.isString(message.query))
                    return "query: string expected";
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                if (!$util.isInteger(message.maxResults))
                    return "maxResults: integer expected";
            return null;
        };

        /**
         * Creates a SearchPageNamesRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchPageNamesRequest} SearchPageNamesRequest
         */
        SearchPageNamesRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchPageNamesRequest)
                return object;
            let message = new $root.petajournal.SearchPageNamesRequest();
            if (object.query != null)
                message.query = String(object.query);
            if (object.maxResults != null)
                message.maxResults = object.maxResults | 0;
            return message;
        };

        /**
         * Creates a plain object from a SearchPageNamesRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchPageNamesRequest
         * @static
         * @param {petajournal.SearchPageNamesRequest} message SearchPageNamesRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchPageNamesRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.query = "";
                object.maxResults = 0;
            }
            if (message.query != null && message.hasOwnProperty("query"))
                object.query = message.query;
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                object.maxResults = message.maxResults;
            return object;
        };

        /**
         * Converts this SearchPageNamesRequest to JSON.
         * @function toJSON
         * @memberof petajournal.SearchPageNamesRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchPageNamesRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchPageNamesResponse message, length delimited. Does not implicitly {@link petajournal.SearchPageNamesResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {petajournal.ISearchPageNamesResponse} message SearchPageNamesResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchPageNamesResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchPageNamesResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchPageNamesResponse} SearchPageNamesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchPageNamesResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchPageNamesResponse message.
         * @function verify
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchPageNamesResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.results != null && message.hasOwnProperty("results")) {
                if (!Array.isArray(message.results))
                    return "results: array expected";
                for (let i = 0; i < message.results.length; ++i) {
                    let error = $root.petajournal.SearchResult.verify(message.results[i]);
                    if (error)
                        return "results." + error;
                }
            }
            return null;
        };

        /**
         * Creates a SearchPageNamesResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchPageNamesResponse} SearchPageNamesResponse
         */
        SearchPageNamesResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchPageNamesResponse)
                return object;
            let message = new $root.petajournal.SearchPageNamesResponse();
            if (object.results) {
                if (!Array.isArray(object.results))
                    throw TypeError(".petajournal.SearchPageNamesResponse.results: array expected");
                message.results = [];
                for (let i = 0; i < object.results.length; ++i) {
                    if (typeof object.results[i] !== "object")
                        throw TypeError(".petajournal.SearchPageNamesResponse.results: object expected");
                    message.results[i] = $root.petajournal.SearchResult.fromObject(object.results[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a SearchPageNamesResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchPageNamesResponse
         * @static
         * @param {petajournal.SearchPageNamesResponse} message SearchPageNamesResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchPageNamesResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.results = [];
            if (message.results && message.results.length) {
                object.results = [];
                for (let j = 0; j < message.results.length; ++j)
                    object.results[j] = $root.petajournal.SearchResult.toObject(message.results[j], options);
            }
            return object;
        };

        /**
         * Converts this SearchPageNamesResponse to JSON.
         * @function toJSON
         * @memberof petajournal.SearchPageNamesResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchPageNamesResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchFullTextRequest message, length delimited. Does not implicitly {@link petajournal.SearchFullTextRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {petajournal.ISearchFullTextRequest} message SearchFullTextRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchFullTextRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchFullTextRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchFullTextRequest} SearchFullTextRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchFullTextRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchFullTextRequest message.
         * @function verify
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchFullTextRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.query != null && message.hasOwnProperty("query"))
                if (!$util.isString(message.query))
                    return "query: string expected";
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                if (!$util.isInteger(message.maxResults))
                    return "maxResults: integer expected";
            return null;
        };

        /**
         * Creates a SearchFullTextRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchFullTextRequest} SearchFullTextRequest
         */
        SearchFullTextRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchFullTextRequest)
                return object;
            let message = new $root.petajournal.SearchFullTextRequest();
            if (object.query != null)
                message.query = String(object.query);
            if (object.maxResults != null)
                message.maxResults = object.maxResults | 0;
            return message;
        };

        /**
         * Creates a plain object from a SearchFullTextRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchFullTextRequest
         * @static
         * @param {petajournal.SearchFullTextRequest} message SearchFullTextRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchFullTextRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.query = "";
                object.maxResults = 0;
            }
            if (message.query != null && message.hasOwnProperty("query"))
                object.query = message.query;
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                object.maxResults = message.maxResults;
            return object;
        };

        /**
         * Converts this SearchFullTextRequest to JSON.
         * @function toJSON
         * @memberof petajournal.SearchFullTextRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchFullTextRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified SearchFullTextResponse message, length delimited. Does not implicitly {@link petajournal.SearchFullTextResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {petajournal.ISearchFullTextResponse} message SearchFullTextResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SearchFullTextResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a SearchFullTextResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.SearchFullTextResponse} SearchFullTextResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SearchFullTextResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SearchFullTextResponse message.
         * @function verify
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SearchFullTextResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.results != null && message.hasOwnProperty("results")) {
                if (!Array.isArray(message.results))
                    return "results: array expected";
                for (let i = 0; i < message.results.length; ++i) {
                    let error = $root.petajournal.SearchResult.verify(message.results[i]);
                    if (error)
                        return "results." + error;
                }
            }
            if (message.totalHits != null && message.hasOwnProperty("totalHits"))
                if (!$util.isInteger(message.totalHits))
                    return "totalHits: integer expected";
            return null;
        };

        /**
         * Creates a SearchFullTextResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.SearchFullTextResponse} SearchFullTextResponse
         */
        SearchFullTextResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.SearchFullTextResponse)
                return object;
            let message = new $root.petajournal.SearchFullTextResponse();
            if (object.results) {
                if (!Array.isArray(object.results))
                    throw TypeError(".petajournal.SearchFullTextResponse.results: array expected");
                message.results = [];
                for (let i = 0; i < object.results.length; ++i) {
                    if (typeof object.results[i] !== "object")
                        throw TypeError(".petajournal.SearchFullTextResponse.results: object expected");
                    message.results[i] = $root.petajournal.SearchResult.fromObject(object.results[i]);
                }
            }
            if (object.totalHits != null)
                message.totalHits = object.totalHits | 0;
            return message;
        };

        /**
         * Creates a plain object from a SearchFullTextResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.SearchFullTextResponse
         * @static
         * @param {petajournal.SearchFullTextResponse} message SearchFullTextResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SearchFullTextResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.results = [];
            if (options.defaults)
                object.totalHits = 0;
            if (message.results && message.results.length) {
                object.results = [];
                for (let j = 0; j < message.results.length; ++j)
                    object.results[j] = $root.petajournal.SearchResult.toObject(message.results[j], options);
            }
            if (message.totalHits != null && message.hasOwnProperty("totalHits"))
                object.totalHits = message.totalHits;
            return object;
        };

        /**
         * Converts this SearchFullTextResponse to JSON.
         * @function toJSON
         * @memberof petajournal.SearchFullTextResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SearchFullTextResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * @property {string|null} [type] GetRecentPagesRequest type
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
         * GetRecentPagesRequest type.
         * @member {string} type
         * @memberof petajournal.GetRecentPagesRequest
         * @instance
         */
        GetRecentPagesRequest.prototype.type = "";

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
            if (message.type != null && Object.hasOwnProperty.call(message, "type"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.type);
            return writer;
        };

        /**
         * Encodes the specified GetRecentPagesRequest message, length delimited. Does not implicitly {@link petajournal.GetRecentPagesRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {petajournal.IGetRecentPagesRequest} message GetRecentPagesRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetRecentPagesRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
                case 2: {
                        message.type = reader.string();
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
         * Decodes a GetRecentPagesRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetRecentPagesRequest} GetRecentPagesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetRecentPagesRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetRecentPagesRequest message.
         * @function verify
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetRecentPagesRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                if (!$util.isInteger(message.maxResults))
                    return "maxResults: integer expected";
            if (message.type != null && message.hasOwnProperty("type"))
                if (!$util.isString(message.type))
                    return "type: string expected";
            return null;
        };

        /**
         * Creates a GetRecentPagesRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetRecentPagesRequest} GetRecentPagesRequest
         */
        GetRecentPagesRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetRecentPagesRequest)
                return object;
            let message = new $root.petajournal.GetRecentPagesRequest();
            if (object.maxResults != null)
                message.maxResults = object.maxResults | 0;
            if (object.type != null)
                message.type = String(object.type);
            return message;
        };

        /**
         * Creates a plain object from a GetRecentPagesRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetRecentPagesRequest
         * @static
         * @param {petajournal.GetRecentPagesRequest} message GetRecentPagesRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetRecentPagesRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.maxResults = 0;
                object.type = "";
            }
            if (message.maxResults != null && message.hasOwnProperty("maxResults"))
                object.maxResults = message.maxResults;
            if (message.type != null && message.hasOwnProperty("type"))
                object.type = message.type;
            return object;
        };

        /**
         * Converts this GetRecentPagesRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetRecentPagesRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetRecentPagesRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetRecentPagesResponse message, length delimited. Does not implicitly {@link petajournal.GetRecentPagesResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {petajournal.IGetRecentPagesResponse} message GetRecentPagesResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetRecentPagesResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetRecentPagesResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetRecentPagesResponse} GetRecentPagesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetRecentPagesResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetRecentPagesResponse message.
         * @function verify
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetRecentPagesResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pages != null && message.hasOwnProperty("pages")) {
                if (!Array.isArray(message.pages))
                    return "pages: array expected";
                for (let i = 0; i < message.pages.length; ++i) {
                    let error = $root.petajournal.Page.verify(message.pages[i]);
                    if (error)
                        return "pages." + error;
                }
            }
            return null;
        };

        /**
         * Creates a GetRecentPagesResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetRecentPagesResponse} GetRecentPagesResponse
         */
        GetRecentPagesResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetRecentPagesResponse)
                return object;
            let message = new $root.petajournal.GetRecentPagesResponse();
            if (object.pages) {
                if (!Array.isArray(object.pages))
                    throw TypeError(".petajournal.GetRecentPagesResponse.pages: array expected");
                message.pages = [];
                for (let i = 0; i < object.pages.length; ++i) {
                    if (typeof object.pages[i] !== "object")
                        throw TypeError(".petajournal.GetRecentPagesResponse.pages: object expected");
                    message.pages[i] = $root.petajournal.Page.fromObject(object.pages[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a GetRecentPagesResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetRecentPagesResponse
         * @static
         * @param {petajournal.GetRecentPagesResponse} message GetRecentPagesResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetRecentPagesResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.pages = [];
            if (message.pages && message.pages.length) {
                object.pages = [];
                for (let j = 0; j < message.pages.length; ++j)
                    object.pages[j] = $root.petajournal.Page.toObject(message.pages[j], options);
            }
            return object;
        };

        /**
         * Converts this GetRecentPagesResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetRecentPagesResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetRecentPagesResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetBacklinksRequest message, length delimited. Does not implicitly {@link petajournal.GetBacklinksRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {petajournal.IGetBacklinksRequest} message GetBacklinksRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBacklinksRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetBacklinksRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetBacklinksRequest} GetBacklinksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBacklinksRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetBacklinksRequest message.
         * @function verify
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetBacklinksRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            return null;
        };

        /**
         * Creates a GetBacklinksRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetBacklinksRequest} GetBacklinksRequest
         */
        GetBacklinksRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetBacklinksRequest)
                return object;
            let message = new $root.petajournal.GetBacklinksRequest();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            return message;
        };

        /**
         * Creates a plain object from a GetBacklinksRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetBacklinksRequest
         * @static
         * @param {petajournal.GetBacklinksRequest} message GetBacklinksRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetBacklinksRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.pageId = "";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            return object;
        };

        /**
         * Converts this GetBacklinksRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetBacklinksRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetBacklinksRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetBacklinksResponse message, length delimited. Does not implicitly {@link petajournal.GetBacklinksResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {petajournal.IGetBacklinksResponse} message GetBacklinksResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBacklinksResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetBacklinksResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetBacklinksResponse} GetBacklinksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBacklinksResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetBacklinksResponse message.
         * @function verify
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetBacklinksResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.entries != null && message.hasOwnProperty("entries")) {
                if (!Array.isArray(message.entries))
                    return "entries: array expected";
                for (let i = 0; i < message.entries.length; ++i) {
                    let error = $root.petajournal.BacklinkEntry.verify(message.entries[i]);
                    if (error)
                        return "entries." + error;
                }
            }
            return null;
        };

        /**
         * Creates a GetBacklinksResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetBacklinksResponse} GetBacklinksResponse
         */
        GetBacklinksResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetBacklinksResponse)
                return object;
            let message = new $root.petajournal.GetBacklinksResponse();
            if (object.entries) {
                if (!Array.isArray(object.entries))
                    throw TypeError(".petajournal.GetBacklinksResponse.entries: array expected");
                message.entries = [];
                for (let i = 0; i < object.entries.length; ++i) {
                    if (typeof object.entries[i] !== "object")
                        throw TypeError(".petajournal.GetBacklinksResponse.entries: object expected");
                    message.entries[i] = $root.petajournal.BacklinkEntry.fromObject(object.entries[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a GetBacklinksResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetBacklinksResponse
         * @static
         * @param {petajournal.GetBacklinksResponse} message GetBacklinksResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetBacklinksResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.entries = [];
            if (message.entries && message.entries.length) {
                object.entries = [];
                for (let j = 0; j < message.entries.length; ++j)
                    object.entries[j] = $root.petajournal.BacklinkEntry.toObject(message.entries[j], options);
            }
            return object;
        };

        /**
         * Converts this GetBacklinksResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetBacklinksResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetBacklinksResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified BacklinkEntry message, length delimited. Does not implicitly {@link petajournal.BacklinkEntry.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {petajournal.IBacklinkEntry} message BacklinkEntry message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        BacklinkEntry.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a BacklinkEntry message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.BacklinkEntry} BacklinkEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        BacklinkEntry.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a BacklinkEntry message.
         * @function verify
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        BacklinkEntry.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.sourcePage != null && message.hasOwnProperty("sourcePage")) {
                let error = $root.petajournal.Page.verify(message.sourcePage);
                if (error)
                    return "sourcePage." + error;
            }
            if (message.sourceBlock != null && message.hasOwnProperty("sourceBlock")) {
                let error = $root.petajournal.Block.verify(message.sourceBlock);
                if (error)
                    return "sourceBlock." + error;
            }
            return null;
        };

        /**
         * Creates a BacklinkEntry message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.BacklinkEntry} BacklinkEntry
         */
        BacklinkEntry.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.BacklinkEntry)
                return object;
            let message = new $root.petajournal.BacklinkEntry();
            if (object.sourcePage != null) {
                if (typeof object.sourcePage !== "object")
                    throw TypeError(".petajournal.BacklinkEntry.sourcePage: object expected");
                message.sourcePage = $root.petajournal.Page.fromObject(object.sourcePage);
            }
            if (object.sourceBlock != null) {
                if (typeof object.sourceBlock !== "object")
                    throw TypeError(".petajournal.BacklinkEntry.sourceBlock: object expected");
                message.sourceBlock = $root.petajournal.Block.fromObject(object.sourceBlock);
            }
            return message;
        };

        /**
         * Creates a plain object from a BacklinkEntry message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.BacklinkEntry
         * @static
         * @param {petajournal.BacklinkEntry} message BacklinkEntry
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        BacklinkEntry.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.sourcePage = null;
                object.sourceBlock = null;
            }
            if (message.sourcePage != null && message.hasOwnProperty("sourcePage"))
                object.sourcePage = $root.petajournal.Page.toObject(message.sourcePage, options);
            if (message.sourceBlock != null && message.hasOwnProperty("sourceBlock"))
                object.sourceBlock = $root.petajournal.Block.toObject(message.sourceBlock, options);
            return object;
        };

        /**
         * Converts this BacklinkEntry to JSON.
         * @function toJSON
         * @memberof petajournal.BacklinkEntry
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        BacklinkEntry.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetMostRecentJournalBeforeRequest message, length delimited. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {petajournal.IGetMostRecentJournalBeforeRequest} message GetMostRecentJournalBeforeRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetMostRecentJournalBeforeRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetMostRecentJournalBeforeRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetMostRecentJournalBeforeRequest} GetMostRecentJournalBeforeRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetMostRecentJournalBeforeRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetMostRecentJournalBeforeRequest message.
         * @function verify
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetMostRecentJournalBeforeRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.beforeDate != null && message.hasOwnProperty("beforeDate"))
                if (!$util.isString(message.beforeDate))
                    return "beforeDate: string expected";
            return null;
        };

        /**
         * Creates a GetMostRecentJournalBeforeRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetMostRecentJournalBeforeRequest} GetMostRecentJournalBeforeRequest
         */
        GetMostRecentJournalBeforeRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetMostRecentJournalBeforeRequest)
                return object;
            let message = new $root.petajournal.GetMostRecentJournalBeforeRequest();
            if (object.beforeDate != null)
                message.beforeDate = String(object.beforeDate);
            return message;
        };

        /**
         * Creates a plain object from a GetMostRecentJournalBeforeRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @static
         * @param {petajournal.GetMostRecentJournalBeforeRequest} message GetMostRecentJournalBeforeRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetMostRecentJournalBeforeRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.beforeDate = "";
            if (message.beforeDate != null && message.hasOwnProperty("beforeDate"))
                object.beforeDate = message.beforeDate;
            return object;
        };

        /**
         * Converts this GetMostRecentJournalBeforeRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetMostRecentJournalBeforeRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetMostRecentJournalBeforeRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetMostRecentJournalBeforeResponse message, length delimited. Does not implicitly {@link petajournal.GetMostRecentJournalBeforeResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {petajournal.IGetMostRecentJournalBeforeResponse} message GetMostRecentJournalBeforeResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetMostRecentJournalBeforeResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetMostRecentJournalBeforeResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetMostRecentJournalBeforeResponse} GetMostRecentJournalBeforeResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetMostRecentJournalBeforeResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetMostRecentJournalBeforeResponse message.
         * @function verify
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetMostRecentJournalBeforeResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.found != null && message.hasOwnProperty("found"))
                if (typeof message.found !== "boolean")
                    return "found: boolean expected";
            if (message.page != null && message.hasOwnProperty("page")) {
                let error = $root.petajournal.Page.verify(message.page);
                if (error)
                    return "page." + error;
            }
            if (message.blocks != null && message.hasOwnProperty("blocks")) {
                if (!Array.isArray(message.blocks))
                    return "blocks: array expected";
                for (let i = 0; i < message.blocks.length; ++i) {
                    let error = $root.petajournal.Block.verify(message.blocks[i]);
                    if (error)
                        return "blocks." + error;
                }
            }
            return null;
        };

        /**
         * Creates a GetMostRecentJournalBeforeResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetMostRecentJournalBeforeResponse} GetMostRecentJournalBeforeResponse
         */
        GetMostRecentJournalBeforeResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetMostRecentJournalBeforeResponse)
                return object;
            let message = new $root.petajournal.GetMostRecentJournalBeforeResponse();
            if (object.found != null)
                message.found = Boolean(object.found);
            if (object.page != null) {
                if (typeof object.page !== "object")
                    throw TypeError(".petajournal.GetMostRecentJournalBeforeResponse.page: object expected");
                message.page = $root.petajournal.Page.fromObject(object.page);
            }
            if (object.blocks) {
                if (!Array.isArray(object.blocks))
                    throw TypeError(".petajournal.GetMostRecentJournalBeforeResponse.blocks: array expected");
                message.blocks = [];
                for (let i = 0; i < object.blocks.length; ++i) {
                    if (typeof object.blocks[i] !== "object")
                        throw TypeError(".petajournal.GetMostRecentJournalBeforeResponse.blocks: object expected");
                    message.blocks[i] = $root.petajournal.Block.fromObject(object.blocks[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a GetMostRecentJournalBeforeResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @static
         * @param {petajournal.GetMostRecentJournalBeforeResponse} message GetMostRecentJournalBeforeResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetMostRecentJournalBeforeResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blocks = [];
            if (options.defaults) {
                object.found = false;
                object.page = null;
            }
            if (message.found != null && message.hasOwnProperty("found"))
                object.found = message.found;
            if (message.page != null && message.hasOwnProperty("page"))
                object.page = $root.petajournal.Page.toObject(message.page, options);
            if (message.blocks && message.blocks.length) {
                object.blocks = [];
                for (let j = 0; j < message.blocks.length; ++j)
                    object.blocks[j] = $root.petajournal.Block.toObject(message.blocks[j], options);
            }
            return object;
        };

        /**
         * Converts this GetMostRecentJournalBeforeResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetMostRecentJournalBeforeResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetMostRecentJournalBeforeResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetBlocksByStatusRequest message, length delimited. Does not implicitly {@link petajournal.GetBlocksByStatusRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {petajournal.IGetBlocksByStatusRequest} message GetBlocksByStatusRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBlocksByStatusRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetBlocksByStatusRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetBlocksByStatusRequest} GetBlocksByStatusRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBlocksByStatusRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetBlocksByStatusRequest message.
         * @function verify
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetBlocksByStatusRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.statuses != null && message.hasOwnProperty("statuses")) {
                if (!Array.isArray(message.statuses))
                    return "statuses: array expected";
                for (let i = 0; i < message.statuses.length; ++i)
                    if (!$util.isString(message.statuses[i]))
                        return "statuses: string[] expected";
            }
            return null;
        };

        /**
         * Creates a GetBlocksByStatusRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetBlocksByStatusRequest} GetBlocksByStatusRequest
         */
        GetBlocksByStatusRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetBlocksByStatusRequest)
                return object;
            let message = new $root.petajournal.GetBlocksByStatusRequest();
            if (object.statuses) {
                if (!Array.isArray(object.statuses))
                    throw TypeError(".petajournal.GetBlocksByStatusRequest.statuses: array expected");
                message.statuses = [];
                for (let i = 0; i < object.statuses.length; ++i)
                    message.statuses[i] = String(object.statuses[i]);
            }
            return message;
        };

        /**
         * Creates a plain object from a GetBlocksByStatusRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetBlocksByStatusRequest
         * @static
         * @param {petajournal.GetBlocksByStatusRequest} message GetBlocksByStatusRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetBlocksByStatusRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.statuses = [];
            if (message.statuses && message.statuses.length) {
                object.statuses = [];
                for (let j = 0; j < message.statuses.length; ++j)
                    object.statuses[j] = message.statuses[j];
            }
            return object;
        };

        /**
         * Converts this GetBlocksByStatusRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetBlocksByStatusRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetBlocksByStatusRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetBlocksByStatusResponse message, length delimited. Does not implicitly {@link petajournal.GetBlocksByStatusResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {petajournal.IGetBlocksByStatusResponse} message GetBlocksByStatusResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBlocksByStatusResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetBlocksByStatusResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetBlocksByStatusResponse} GetBlocksByStatusResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBlocksByStatusResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetBlocksByStatusResponse message.
         * @function verify
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetBlocksByStatusResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.blocks != null && message.hasOwnProperty("blocks")) {
                if (!Array.isArray(message.blocks))
                    return "blocks: array expected";
                for (let i = 0; i < message.blocks.length; ++i) {
                    let error = $root.petajournal.Block.verify(message.blocks[i]);
                    if (error)
                        return "blocks." + error;
                }
            }
            return null;
        };

        /**
         * Creates a GetBlocksByStatusResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetBlocksByStatusResponse} GetBlocksByStatusResponse
         */
        GetBlocksByStatusResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetBlocksByStatusResponse)
                return object;
            let message = new $root.petajournal.GetBlocksByStatusResponse();
            if (object.blocks) {
                if (!Array.isArray(object.blocks))
                    throw TypeError(".petajournal.GetBlocksByStatusResponse.blocks: array expected");
                message.blocks = [];
                for (let i = 0; i < object.blocks.length; ++i) {
                    if (typeof object.blocks[i] !== "object")
                        throw TypeError(".petajournal.GetBlocksByStatusResponse.blocks: object expected");
                    message.blocks[i] = $root.petajournal.Block.fromObject(object.blocks[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a GetBlocksByStatusResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetBlocksByStatusResponse
         * @static
         * @param {petajournal.GetBlocksByStatusResponse} message GetBlocksByStatusResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetBlocksByStatusResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blocks = [];
            if (message.blocks && message.blocks.length) {
                object.blocks = [];
                for (let j = 0; j < message.blocks.length; ++j)
                    object.blocks[j] = $root.petajournal.Block.toObject(message.blocks[j], options);
            }
            return object;
        };

        /**
         * Converts this GetBlocksByStatusResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetBlocksByStatusResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetBlocksByStatusResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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

    petajournal.GetOrCreateJournalRequest = (function() {

        /**
         * Properties of a GetOrCreateJournalRequest.
         * @memberof petajournal
         * @interface IGetOrCreateJournalRequest
         * @property {string|null} [name] GetOrCreateJournalRequest name
         */

        /**
         * Constructs a new GetOrCreateJournalRequest.
         * @memberof petajournal
         * @classdesc Represents a GetOrCreateJournalRequest.
         * @implements IGetOrCreateJournalRequest
         * @constructor
         * @param {petajournal.IGetOrCreateJournalRequest=} [properties] Properties to set
         */
        function GetOrCreateJournalRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetOrCreateJournalRequest name.
         * @member {string} name
         * @memberof petajournal.GetOrCreateJournalRequest
         * @instance
         */
        GetOrCreateJournalRequest.prototype.name = "";

        /**
         * Creates a new GetOrCreateJournalRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {petajournal.IGetOrCreateJournalRequest=} [properties] Properties to set
         * @returns {petajournal.GetOrCreateJournalRequest} GetOrCreateJournalRequest instance
         */
        GetOrCreateJournalRequest.create = function create(properties) {
            return new GetOrCreateJournalRequest(properties);
        };

        /**
         * Encodes the specified GetOrCreateJournalRequest message. Does not implicitly {@link petajournal.GetOrCreateJournalRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {petajournal.IGetOrCreateJournalRequest} message GetOrCreateJournalRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateJournalRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.name);
            return writer;
        };

        /**
         * Encodes the specified GetOrCreateJournalRequest message, length delimited. Does not implicitly {@link petajournal.GetOrCreateJournalRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {petajournal.IGetOrCreateJournalRequest} message GetOrCreateJournalRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateJournalRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetOrCreateJournalRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetOrCreateJournalRequest} GetOrCreateJournalRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateJournalRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetOrCreateJournalRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.name = reader.string();
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
         * Decodes a GetOrCreateJournalRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetOrCreateJournalRequest} GetOrCreateJournalRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateJournalRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetOrCreateJournalRequest message.
         * @function verify
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetOrCreateJournalRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.name != null && message.hasOwnProperty("name"))
                if (!$util.isString(message.name))
                    return "name: string expected";
            return null;
        };

        /**
         * Creates a GetOrCreateJournalRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetOrCreateJournalRequest} GetOrCreateJournalRequest
         */
        GetOrCreateJournalRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetOrCreateJournalRequest)
                return object;
            let message = new $root.petajournal.GetOrCreateJournalRequest();
            if (object.name != null)
                message.name = String(object.name);
            return message;
        };

        /**
         * Creates a plain object from a GetOrCreateJournalRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {petajournal.GetOrCreateJournalRequest} message GetOrCreateJournalRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetOrCreateJournalRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.name = "";
            if (message.name != null && message.hasOwnProperty("name"))
                object.name = message.name;
            return object;
        };

        /**
         * Converts this GetOrCreateJournalRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetOrCreateJournalRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetOrCreateJournalRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetOrCreateJournalRequest
         * @function getTypeUrl
         * @memberof petajournal.GetOrCreateJournalRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetOrCreateJournalRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetOrCreateJournalRequest";
        };

        return GetOrCreateJournalRequest;
    })();

    petajournal.GetOrCreateJournalResponse = (function() {

        /**
         * Properties of a GetOrCreateJournalResponse.
         * @memberof petajournal
         * @interface IGetOrCreateJournalResponse
         * @property {petajournal.IPage|null} [page] GetOrCreateJournalResponse page
         * @property {Array.<petajournal.IBlock>|null} [blocks] GetOrCreateJournalResponse blocks
         * @property {boolean|null} [created] GetOrCreateJournalResponse created
         */

        /**
         * Constructs a new GetOrCreateJournalResponse.
         * @memberof petajournal
         * @classdesc Represents a GetOrCreateJournalResponse.
         * @implements IGetOrCreateJournalResponse
         * @constructor
         * @param {petajournal.IGetOrCreateJournalResponse=} [properties] Properties to set
         */
        function GetOrCreateJournalResponse(properties) {
            this.blocks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetOrCreateJournalResponse page.
         * @member {petajournal.IPage|null|undefined} page
         * @memberof petajournal.GetOrCreateJournalResponse
         * @instance
         */
        GetOrCreateJournalResponse.prototype.page = null;

        /**
         * GetOrCreateJournalResponse blocks.
         * @member {Array.<petajournal.IBlock>} blocks
         * @memberof petajournal.GetOrCreateJournalResponse
         * @instance
         */
        GetOrCreateJournalResponse.prototype.blocks = $util.emptyArray;

        /**
         * GetOrCreateJournalResponse created.
         * @member {boolean} created
         * @memberof petajournal.GetOrCreateJournalResponse
         * @instance
         */
        GetOrCreateJournalResponse.prototype.created = false;

        /**
         * Creates a new GetOrCreateJournalResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {petajournal.IGetOrCreateJournalResponse=} [properties] Properties to set
         * @returns {petajournal.GetOrCreateJournalResponse} GetOrCreateJournalResponse instance
         */
        GetOrCreateJournalResponse.create = function create(properties) {
            return new GetOrCreateJournalResponse(properties);
        };

        /**
         * Encodes the specified GetOrCreateJournalResponse message. Does not implicitly {@link petajournal.GetOrCreateJournalResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {petajournal.IGetOrCreateJournalResponse} message GetOrCreateJournalResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateJournalResponse.encode = function encode(message, writer) {
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
         * Encodes the specified GetOrCreateJournalResponse message, length delimited. Does not implicitly {@link petajournal.GetOrCreateJournalResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {petajournal.IGetOrCreateJournalResponse} message GetOrCreateJournalResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetOrCreateJournalResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetOrCreateJournalResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetOrCreateJournalResponse} GetOrCreateJournalResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateJournalResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetOrCreateJournalResponse();
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
         * Decodes a GetOrCreateJournalResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetOrCreateJournalResponse} GetOrCreateJournalResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetOrCreateJournalResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetOrCreateJournalResponse message.
         * @function verify
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetOrCreateJournalResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.page != null && message.hasOwnProperty("page")) {
                let error = $root.petajournal.Page.verify(message.page);
                if (error)
                    return "page." + error;
            }
            if (message.blocks != null && message.hasOwnProperty("blocks")) {
                if (!Array.isArray(message.blocks))
                    return "blocks: array expected";
                for (let i = 0; i < message.blocks.length; ++i) {
                    let error = $root.petajournal.Block.verify(message.blocks[i]);
                    if (error)
                        return "blocks." + error;
                }
            }
            if (message.created != null && message.hasOwnProperty("created"))
                if (typeof message.created !== "boolean")
                    return "created: boolean expected";
            return null;
        };

        /**
         * Creates a GetOrCreateJournalResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetOrCreateJournalResponse} GetOrCreateJournalResponse
         */
        GetOrCreateJournalResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetOrCreateJournalResponse)
                return object;
            let message = new $root.petajournal.GetOrCreateJournalResponse();
            if (object.page != null) {
                if (typeof object.page !== "object")
                    throw TypeError(".petajournal.GetOrCreateJournalResponse.page: object expected");
                message.page = $root.petajournal.Page.fromObject(object.page);
            }
            if (object.blocks) {
                if (!Array.isArray(object.blocks))
                    throw TypeError(".petajournal.GetOrCreateJournalResponse.blocks: array expected");
                message.blocks = [];
                for (let i = 0; i < object.blocks.length; ++i) {
                    if (typeof object.blocks[i] !== "object")
                        throw TypeError(".petajournal.GetOrCreateJournalResponse.blocks: object expected");
                    message.blocks[i] = $root.petajournal.Block.fromObject(object.blocks[i]);
                }
            }
            if (object.created != null)
                message.created = Boolean(object.created);
            return message;
        };

        /**
         * Creates a plain object from a GetOrCreateJournalResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {petajournal.GetOrCreateJournalResponse} message GetOrCreateJournalResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetOrCreateJournalResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blocks = [];
            if (options.defaults) {
                object.page = null;
                object.created = false;
            }
            if (message.page != null && message.hasOwnProperty("page"))
                object.page = $root.petajournal.Page.toObject(message.page, options);
            if (message.blocks && message.blocks.length) {
                object.blocks = [];
                for (let j = 0; j < message.blocks.length; ++j)
                    object.blocks[j] = $root.petajournal.Block.toObject(message.blocks[j], options);
            }
            if (message.created != null && message.hasOwnProperty("created"))
                object.created = message.created;
            return object;
        };

        /**
         * Converts this GetOrCreateJournalResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetOrCreateJournalResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetOrCreateJournalResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetOrCreateJournalResponse
         * @function getTypeUrl
         * @memberof petajournal.GetOrCreateJournalResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetOrCreateJournalResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetOrCreateJournalResponse";
        };

        return GetOrCreateJournalResponse;
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
         * Encodes the specified GetConfigRequest message, length delimited. Does not implicitly {@link petajournal.GetConfigRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {petajournal.IGetConfigRequest} message GetConfigRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetConfigRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetConfigRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetConfigRequest} GetConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetConfigRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetConfigRequest message.
         * @function verify
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetConfigRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            return null;
        };

        /**
         * Creates a GetConfigRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetConfigRequest} GetConfigRequest
         */
        GetConfigRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetConfigRequest)
                return object;
            return new $root.petajournal.GetConfigRequest();
        };

        /**
         * Creates a plain object from a GetConfigRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetConfigRequest
         * @static
         * @param {petajournal.GetConfigRequest} message GetConfigRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetConfigRequest.toObject = function toObject() {
            return {};
        };

        /**
         * Converts this GetConfigRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetConfigRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetConfigRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified GetConfigResponse message, length delimited. Does not implicitly {@link petajournal.GetConfigResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {petajournal.IGetConfigResponse} message GetConfigResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetConfigResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a GetConfigResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetConfigResponse} GetConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetConfigResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetConfigResponse message.
         * @function verify
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetConfigResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.weekNumbering != null && message.hasOwnProperty("weekNumbering"))
                if (!$util.isString(message.weekNumbering))
                    return "weekNumbering: string expected";
            return null;
        };

        /**
         * Creates a GetConfigResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetConfigResponse} GetConfigResponse
         */
        GetConfigResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetConfigResponse)
                return object;
            let message = new $root.petajournal.GetConfigResponse();
            if (object.weekNumbering != null)
                message.weekNumbering = String(object.weekNumbering);
            return message;
        };

        /**
         * Creates a plain object from a GetConfigResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetConfigResponse
         * @static
         * @param {petajournal.GetConfigResponse} message GetConfigResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetConfigResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.weekNumbering = "";
            if (message.weekNumbering != null && message.hasOwnProperty("weekNumbering"))
                object.weekNumbering = message.weekNumbering;
            return object;
        };

        /**
         * Converts this GetConfigResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetConfigResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetConfigResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified UpdateConfigRequest message, length delimited. Does not implicitly {@link petajournal.UpdateConfigRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {petajournal.IUpdateConfigRequest} message UpdateConfigRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateConfigRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an UpdateConfigRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateConfigRequest} UpdateConfigRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateConfigRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateConfigRequest message.
         * @function verify
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateConfigRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.weekNumbering != null && message.hasOwnProperty("weekNumbering"))
                if (!$util.isString(message.weekNumbering))
                    return "weekNumbering: string expected";
            return null;
        };

        /**
         * Creates an UpdateConfigRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateConfigRequest} UpdateConfigRequest
         */
        UpdateConfigRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateConfigRequest)
                return object;
            let message = new $root.petajournal.UpdateConfigRequest();
            if (object.weekNumbering != null)
                message.weekNumbering = String(object.weekNumbering);
            return message;
        };

        /**
         * Creates a plain object from an UpdateConfigRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateConfigRequest
         * @static
         * @param {petajournal.UpdateConfigRequest} message UpdateConfigRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateConfigRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.weekNumbering = "";
            if (message.weekNumbering != null && message.hasOwnProperty("weekNumbering"))
                object.weekNumbering = message.weekNumbering;
            return object;
        };

        /**
         * Converts this UpdateConfigRequest to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateConfigRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateConfigRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified UpdateConfigResponse message, length delimited. Does not implicitly {@link petajournal.UpdateConfigResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {petajournal.IUpdateConfigResponse} message UpdateConfigResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateConfigResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an UpdateConfigResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateConfigResponse} UpdateConfigResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateConfigResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateConfigResponse message.
         * @function verify
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateConfigResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.weekNumbering != null && message.hasOwnProperty("weekNumbering"))
                if (!$util.isString(message.weekNumbering))
                    return "weekNumbering: string expected";
            return null;
        };

        /**
         * Creates an UpdateConfigResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateConfigResponse} UpdateConfigResponse
         */
        UpdateConfigResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateConfigResponse)
                return object;
            let message = new $root.petajournal.UpdateConfigResponse();
            if (object.weekNumbering != null)
                message.weekNumbering = String(object.weekNumbering);
            return message;
        };

        /**
         * Creates a plain object from an UpdateConfigResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateConfigResponse
         * @static
         * @param {petajournal.UpdateConfigResponse} message UpdateConfigResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateConfigResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.weekNumbering = "";
            if (message.weekNumbering != null && message.hasOwnProperty("weekNumbering"))
                object.weekNumbering = message.weekNumbering;
            return object;
        };

        /**
         * Converts this UpdateConfigResponse to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateConfigResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateConfigResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified ExportTreeMarkdownRequest message, length delimited. Does not implicitly {@link petajournal.ExportTreeMarkdownRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {petajournal.IExportTreeMarkdownRequest} message ExportTreeMarkdownRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ExportTreeMarkdownRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an ExportTreeMarkdownRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ExportTreeMarkdownRequest} ExportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ExportTreeMarkdownRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an ExportTreeMarkdownRequest message.
         * @function verify
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ExportTreeMarkdownRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.blockIds != null && message.hasOwnProperty("blockIds")) {
                if (!Array.isArray(message.blockIds))
                    return "blockIds: array expected";
                for (let i = 0; i < message.blockIds.length; ++i)
                    if (!$util.isString(message.blockIds[i]))
                        return "blockIds: string[] expected";
            }
            return null;
        };

        /**
         * Creates an ExportTreeMarkdownRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ExportTreeMarkdownRequest} ExportTreeMarkdownRequest
         */
        ExportTreeMarkdownRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ExportTreeMarkdownRequest)
                return object;
            let message = new $root.petajournal.ExportTreeMarkdownRequest();
            if (object.blockIds) {
                if (!Array.isArray(object.blockIds))
                    throw TypeError(".petajournal.ExportTreeMarkdownRequest.blockIds: array expected");
                message.blockIds = [];
                for (let i = 0; i < object.blockIds.length; ++i)
                    message.blockIds[i] = String(object.blockIds[i]);
            }
            return message;
        };

        /**
         * Creates a plain object from an ExportTreeMarkdownRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @static
         * @param {petajournal.ExportTreeMarkdownRequest} message ExportTreeMarkdownRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ExportTreeMarkdownRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults)
                object.blockIds = [];
            if (message.blockIds && message.blockIds.length) {
                object.blockIds = [];
                for (let j = 0; j < message.blockIds.length; ++j)
                    object.blockIds[j] = message.blockIds[j];
            }
            return object;
        };

        /**
         * Converts this ExportTreeMarkdownRequest to JSON.
         * @function toJSON
         * @memberof petajournal.ExportTreeMarkdownRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ExportTreeMarkdownRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified ExportTreeMarkdownResponse message, length delimited. Does not implicitly {@link petajournal.ExportTreeMarkdownResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {petajournal.IExportTreeMarkdownResponse} message ExportTreeMarkdownResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ExportTreeMarkdownResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an ExportTreeMarkdownResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ExportTreeMarkdownResponse} ExportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ExportTreeMarkdownResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an ExportTreeMarkdownResponse message.
         * @function verify
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ExportTreeMarkdownResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.treeMarkdown != null && message.hasOwnProperty("treeMarkdown"))
                if (!$util.isString(message.treeMarkdown))
                    return "treeMarkdown: string expected";
            if (message.html != null && message.hasOwnProperty("html"))
                if (!$util.isString(message.html))
                    return "html: string expected";
            if (message.flatMarkdown != null && message.hasOwnProperty("flatMarkdown"))
                if (!$util.isString(message.flatMarkdown))
                    return "flatMarkdown: string expected";
            return null;
        };

        /**
         * Creates an ExportTreeMarkdownResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ExportTreeMarkdownResponse} ExportTreeMarkdownResponse
         */
        ExportTreeMarkdownResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ExportTreeMarkdownResponse)
                return object;
            let message = new $root.petajournal.ExportTreeMarkdownResponse();
            if (object.treeMarkdown != null)
                message.treeMarkdown = String(object.treeMarkdown);
            if (object.html != null)
                message.html = String(object.html);
            if (object.flatMarkdown != null)
                message.flatMarkdown = String(object.flatMarkdown);
            return message;
        };

        /**
         * Creates a plain object from an ExportTreeMarkdownResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @static
         * @param {petajournal.ExportTreeMarkdownResponse} message ExportTreeMarkdownResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ExportTreeMarkdownResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.treeMarkdown = "";
                object.html = "";
                object.flatMarkdown = "";
            }
            if (message.treeMarkdown != null && message.hasOwnProperty("treeMarkdown"))
                object.treeMarkdown = message.treeMarkdown;
            if (message.html != null && message.hasOwnProperty("html"))
                object.html = message.html;
            if (message.flatMarkdown != null && message.hasOwnProperty("flatMarkdown"))
                object.flatMarkdown = message.flatMarkdown;
            return object;
        };

        /**
         * Converts this ExportTreeMarkdownResponse to JSON.
         * @function toJSON
         * @memberof petajournal.ExportTreeMarkdownResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ExportTreeMarkdownResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified ImportTreeMarkdownRequest message, length delimited. Does not implicitly {@link petajournal.ImportTreeMarkdownRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {petajournal.IImportTreeMarkdownRequest} message ImportTreeMarkdownRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ImportTreeMarkdownRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an ImportTreeMarkdownRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ImportTreeMarkdownRequest} ImportTreeMarkdownRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ImportTreeMarkdownRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an ImportTreeMarkdownRequest message.
         * @function verify
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ImportTreeMarkdownRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.treeMarkdown != null && message.hasOwnProperty("treeMarkdown"))
                if (!$util.isString(message.treeMarkdown))
                    return "treeMarkdown: string expected";
            return null;
        };

        /**
         * Creates an ImportTreeMarkdownRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ImportTreeMarkdownRequest} ImportTreeMarkdownRequest
         */
        ImportTreeMarkdownRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ImportTreeMarkdownRequest)
                return object;
            let message = new $root.petajournal.ImportTreeMarkdownRequest();
            if (object.treeMarkdown != null)
                message.treeMarkdown = String(object.treeMarkdown);
            return message;
        };

        /**
         * Creates a plain object from an ImportTreeMarkdownRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @static
         * @param {petajournal.ImportTreeMarkdownRequest} message ImportTreeMarkdownRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ImportTreeMarkdownRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.treeMarkdown = "";
            if (message.treeMarkdown != null && message.hasOwnProperty("treeMarkdown"))
                object.treeMarkdown = message.treeMarkdown;
            return object;
        };

        /**
         * Converts this ImportTreeMarkdownRequest to JSON.
         * @function toJSON
         * @memberof petajournal.ImportTreeMarkdownRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ImportTreeMarkdownRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified ImportTreeMarkdownResponse message, length delimited. Does not implicitly {@link petajournal.ImportTreeMarkdownResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {petajournal.IImportTreeMarkdownResponse} message ImportTreeMarkdownResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ImportTreeMarkdownResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes an ImportTreeMarkdownResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ImportTreeMarkdownResponse} ImportTreeMarkdownResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ImportTreeMarkdownResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an ImportTreeMarkdownResponse message.
         * @function verify
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ImportTreeMarkdownResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.flatMarkdown != null && message.hasOwnProperty("flatMarkdown"))
                if (!$util.isString(message.flatMarkdown))
                    return "flatMarkdown: string expected";
            return null;
        };

        /**
         * Creates an ImportTreeMarkdownResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ImportTreeMarkdownResponse} ImportTreeMarkdownResponse
         */
        ImportTreeMarkdownResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ImportTreeMarkdownResponse)
                return object;
            let message = new $root.petajournal.ImportTreeMarkdownResponse();
            if (object.flatMarkdown != null)
                message.flatMarkdown = String(object.flatMarkdown);
            return message;
        };

        /**
         * Creates a plain object from an ImportTreeMarkdownResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @static
         * @param {petajournal.ImportTreeMarkdownResponse} message ImportTreeMarkdownResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ImportTreeMarkdownResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.flatMarkdown = "";
            if (message.flatMarkdown != null && message.hasOwnProperty("flatMarkdown"))
                object.flatMarkdown = message.flatMarkdown;
            return object;
        };

        /**
         * Converts this ImportTreeMarkdownResponse to JSON.
         * @function toJSON
         * @memberof petajournal.ImportTreeMarkdownResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ImportTreeMarkdownResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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

    petajournal.GetBookmarksRequest = (function() {

        /**
         * Properties of a GetBookmarksRequest.
         * @memberof petajournal
         * @interface IGetBookmarksRequest
         */

        /**
         * Constructs a new GetBookmarksRequest.
         * @memberof petajournal
         * @classdesc Represents a GetBookmarksRequest.
         * @implements IGetBookmarksRequest
         * @constructor
         * @param {petajournal.IGetBookmarksRequest=} [properties] Properties to set
         */
        function GetBookmarksRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Creates a new GetBookmarksRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {petajournal.IGetBookmarksRequest=} [properties] Properties to set
         * @returns {petajournal.GetBookmarksRequest} GetBookmarksRequest instance
         */
        GetBookmarksRequest.create = function create(properties) {
            return new GetBookmarksRequest(properties);
        };

        /**
         * Encodes the specified GetBookmarksRequest message. Does not implicitly {@link petajournal.GetBookmarksRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {petajournal.IGetBookmarksRequest} message GetBookmarksRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBookmarksRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            return writer;
        };

        /**
         * Encodes the specified GetBookmarksRequest message, length delimited. Does not implicitly {@link petajournal.GetBookmarksRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {petajournal.IGetBookmarksRequest} message GetBookmarksRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBookmarksRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetBookmarksRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetBookmarksRequest} GetBookmarksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBookmarksRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetBookmarksRequest();
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
         * Decodes a GetBookmarksRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetBookmarksRequest} GetBookmarksRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBookmarksRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetBookmarksRequest message.
         * @function verify
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetBookmarksRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            return null;
        };

        /**
         * Creates a GetBookmarksRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetBookmarksRequest} GetBookmarksRequest
         */
        GetBookmarksRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetBookmarksRequest)
                return object;
            return new $root.petajournal.GetBookmarksRequest();
        };

        /**
         * Creates a plain object from a GetBookmarksRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {petajournal.GetBookmarksRequest} message GetBookmarksRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetBookmarksRequest.toObject = function toObject() {
            return {};
        };

        /**
         * Converts this GetBookmarksRequest to JSON.
         * @function toJSON
         * @memberof petajournal.GetBookmarksRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetBookmarksRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetBookmarksRequest
         * @function getTypeUrl
         * @memberof petajournal.GetBookmarksRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetBookmarksRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetBookmarksRequest";
        };

        return GetBookmarksRequest;
    })();

    petajournal.GetBookmarksResponse = (function() {

        /**
         * Properties of a GetBookmarksResponse.
         * @memberof petajournal
         * @interface IGetBookmarksResponse
         * @property {Array.<petajournal.IBookmarkGroup>|null} [groups] GetBookmarksResponse groups
         * @property {Array.<petajournal.IBookmark>|null} [bookmarks] GetBookmarksResponse bookmarks
         */

        /**
         * Constructs a new GetBookmarksResponse.
         * @memberof petajournal
         * @classdesc Represents a GetBookmarksResponse.
         * @implements IGetBookmarksResponse
         * @constructor
         * @param {petajournal.IGetBookmarksResponse=} [properties] Properties to set
         */
        function GetBookmarksResponse(properties) {
            this.groups = [];
            this.bookmarks = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * GetBookmarksResponse groups.
         * @member {Array.<petajournal.IBookmarkGroup>} groups
         * @memberof petajournal.GetBookmarksResponse
         * @instance
         */
        GetBookmarksResponse.prototype.groups = $util.emptyArray;

        /**
         * GetBookmarksResponse bookmarks.
         * @member {Array.<petajournal.IBookmark>} bookmarks
         * @memberof petajournal.GetBookmarksResponse
         * @instance
         */
        GetBookmarksResponse.prototype.bookmarks = $util.emptyArray;

        /**
         * Creates a new GetBookmarksResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {petajournal.IGetBookmarksResponse=} [properties] Properties to set
         * @returns {petajournal.GetBookmarksResponse} GetBookmarksResponse instance
         */
        GetBookmarksResponse.create = function create(properties) {
            return new GetBookmarksResponse(properties);
        };

        /**
         * Encodes the specified GetBookmarksResponse message. Does not implicitly {@link petajournal.GetBookmarksResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {petajournal.IGetBookmarksResponse} message GetBookmarksResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBookmarksResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.groups != null && message.groups.length)
                for (let i = 0; i < message.groups.length; ++i)
                    $root.petajournal.BookmarkGroup.encode(message.groups[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            if (message.bookmarks != null && message.bookmarks.length)
                for (let i = 0; i < message.bookmarks.length; ++i)
                    $root.petajournal.Bookmark.encode(message.bookmarks[i], writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified GetBookmarksResponse message, length delimited. Does not implicitly {@link petajournal.GetBookmarksResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {petajournal.IGetBookmarksResponse} message GetBookmarksResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        GetBookmarksResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a GetBookmarksResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.GetBookmarksResponse} GetBookmarksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBookmarksResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.GetBookmarksResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.groups && message.groups.length))
                            message.groups = [];
                        message.groups.push($root.petajournal.BookmarkGroup.decode(reader, reader.uint32()));
                        break;
                    }
                case 2: {
                        if (!(message.bookmarks && message.bookmarks.length))
                            message.bookmarks = [];
                        message.bookmarks.push($root.petajournal.Bookmark.decode(reader, reader.uint32()));
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
         * Decodes a GetBookmarksResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.GetBookmarksResponse} GetBookmarksResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        GetBookmarksResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a GetBookmarksResponse message.
         * @function verify
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        GetBookmarksResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.groups != null && message.hasOwnProperty("groups")) {
                if (!Array.isArray(message.groups))
                    return "groups: array expected";
                for (let i = 0; i < message.groups.length; ++i) {
                    let error = $root.petajournal.BookmarkGroup.verify(message.groups[i]);
                    if (error)
                        return "groups." + error;
                }
            }
            if (message.bookmarks != null && message.hasOwnProperty("bookmarks")) {
                if (!Array.isArray(message.bookmarks))
                    return "bookmarks: array expected";
                for (let i = 0; i < message.bookmarks.length; ++i) {
                    let error = $root.petajournal.Bookmark.verify(message.bookmarks[i]);
                    if (error)
                        return "bookmarks." + error;
                }
            }
            return null;
        };

        /**
         * Creates a GetBookmarksResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.GetBookmarksResponse} GetBookmarksResponse
         */
        GetBookmarksResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.GetBookmarksResponse)
                return object;
            let message = new $root.petajournal.GetBookmarksResponse();
            if (object.groups) {
                if (!Array.isArray(object.groups))
                    throw TypeError(".petajournal.GetBookmarksResponse.groups: array expected");
                message.groups = [];
                for (let i = 0; i < object.groups.length; ++i) {
                    if (typeof object.groups[i] !== "object")
                        throw TypeError(".petajournal.GetBookmarksResponse.groups: object expected");
                    message.groups[i] = $root.petajournal.BookmarkGroup.fromObject(object.groups[i]);
                }
            }
            if (object.bookmarks) {
                if (!Array.isArray(object.bookmarks))
                    throw TypeError(".petajournal.GetBookmarksResponse.bookmarks: array expected");
                message.bookmarks = [];
                for (let i = 0; i < object.bookmarks.length; ++i) {
                    if (typeof object.bookmarks[i] !== "object")
                        throw TypeError(".petajournal.GetBookmarksResponse.bookmarks: object expected");
                    message.bookmarks[i] = $root.petajournal.Bookmark.fromObject(object.bookmarks[i]);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a GetBookmarksResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {petajournal.GetBookmarksResponse} message GetBookmarksResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        GetBookmarksResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.arrays || options.defaults) {
                object.groups = [];
                object.bookmarks = [];
            }
            if (message.groups && message.groups.length) {
                object.groups = [];
                for (let j = 0; j < message.groups.length; ++j)
                    object.groups[j] = $root.petajournal.BookmarkGroup.toObject(message.groups[j], options);
            }
            if (message.bookmarks && message.bookmarks.length) {
                object.bookmarks = [];
                for (let j = 0; j < message.bookmarks.length; ++j)
                    object.bookmarks[j] = $root.petajournal.Bookmark.toObject(message.bookmarks[j], options);
            }
            return object;
        };

        /**
         * Converts this GetBookmarksResponse to JSON.
         * @function toJSON
         * @memberof petajournal.GetBookmarksResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        GetBookmarksResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for GetBookmarksResponse
         * @function getTypeUrl
         * @memberof petajournal.GetBookmarksResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        GetBookmarksResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.GetBookmarksResponse";
        };

        return GetBookmarksResponse;
    })();

    petajournal.CreateBookmarkGroupRequest = (function() {

        /**
         * Properties of a CreateBookmarkGroupRequest.
         * @memberof petajournal
         * @interface ICreateBookmarkGroupRequest
         * @property {string|null} [name] CreateBookmarkGroupRequest name
         */

        /**
         * Constructs a new CreateBookmarkGroupRequest.
         * @memberof petajournal
         * @classdesc Represents a CreateBookmarkGroupRequest.
         * @implements ICreateBookmarkGroupRequest
         * @constructor
         * @param {petajournal.ICreateBookmarkGroupRequest=} [properties] Properties to set
         */
        function CreateBookmarkGroupRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * CreateBookmarkGroupRequest name.
         * @member {string} name
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @instance
         */
        CreateBookmarkGroupRequest.prototype.name = "";

        /**
         * Creates a new CreateBookmarkGroupRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {petajournal.ICreateBookmarkGroupRequest=} [properties] Properties to set
         * @returns {petajournal.CreateBookmarkGroupRequest} CreateBookmarkGroupRequest instance
         */
        CreateBookmarkGroupRequest.create = function create(properties) {
            return new CreateBookmarkGroupRequest(properties);
        };

        /**
         * Encodes the specified CreateBookmarkGroupRequest message. Does not implicitly {@link petajournal.CreateBookmarkGroupRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {petajournal.ICreateBookmarkGroupRequest} message CreateBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkGroupRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.name);
            return writer;
        };

        /**
         * Encodes the specified CreateBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkGroupRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {petajournal.ICreateBookmarkGroupRequest} message CreateBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkGroupRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a CreateBookmarkGroupRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.CreateBookmarkGroupRequest} CreateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkGroupRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.CreateBookmarkGroupRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.name = reader.string();
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
         * Decodes a CreateBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.CreateBookmarkGroupRequest} CreateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkGroupRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a CreateBookmarkGroupRequest message.
         * @function verify
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        CreateBookmarkGroupRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.name != null && message.hasOwnProperty("name"))
                if (!$util.isString(message.name))
                    return "name: string expected";
            return null;
        };

        /**
         * Creates a CreateBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.CreateBookmarkGroupRequest} CreateBookmarkGroupRequest
         */
        CreateBookmarkGroupRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.CreateBookmarkGroupRequest)
                return object;
            let message = new $root.petajournal.CreateBookmarkGroupRequest();
            if (object.name != null)
                message.name = String(object.name);
            return message;
        };

        /**
         * Creates a plain object from a CreateBookmarkGroupRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {petajournal.CreateBookmarkGroupRequest} message CreateBookmarkGroupRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        CreateBookmarkGroupRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.name = "";
            if (message.name != null && message.hasOwnProperty("name"))
                object.name = message.name;
            return object;
        };

        /**
         * Converts this CreateBookmarkGroupRequest to JSON.
         * @function toJSON
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        CreateBookmarkGroupRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for CreateBookmarkGroupRequest
         * @function getTypeUrl
         * @memberof petajournal.CreateBookmarkGroupRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        CreateBookmarkGroupRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.CreateBookmarkGroupRequest";
        };

        return CreateBookmarkGroupRequest;
    })();

    petajournal.CreateBookmarkGroupResponse = (function() {

        /**
         * Properties of a CreateBookmarkGroupResponse.
         * @memberof petajournal
         * @interface ICreateBookmarkGroupResponse
         * @property {petajournal.IBookmarkGroup|null} [group] CreateBookmarkGroupResponse group
         */

        /**
         * Constructs a new CreateBookmarkGroupResponse.
         * @memberof petajournal
         * @classdesc Represents a CreateBookmarkGroupResponse.
         * @implements ICreateBookmarkGroupResponse
         * @constructor
         * @param {petajournal.ICreateBookmarkGroupResponse=} [properties] Properties to set
         */
        function CreateBookmarkGroupResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * CreateBookmarkGroupResponse group.
         * @member {petajournal.IBookmarkGroup|null|undefined} group
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @instance
         */
        CreateBookmarkGroupResponse.prototype.group = null;

        /**
         * Creates a new CreateBookmarkGroupResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {petajournal.ICreateBookmarkGroupResponse=} [properties] Properties to set
         * @returns {petajournal.CreateBookmarkGroupResponse} CreateBookmarkGroupResponse instance
         */
        CreateBookmarkGroupResponse.create = function create(properties) {
            return new CreateBookmarkGroupResponse(properties);
        };

        /**
         * Encodes the specified CreateBookmarkGroupResponse message. Does not implicitly {@link petajournal.CreateBookmarkGroupResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {petajournal.ICreateBookmarkGroupResponse} message CreateBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkGroupResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.group != null && Object.hasOwnProperty.call(message, "group"))
                $root.petajournal.BookmarkGroup.encode(message.group, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified CreateBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkGroupResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {petajournal.ICreateBookmarkGroupResponse} message CreateBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkGroupResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a CreateBookmarkGroupResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.CreateBookmarkGroupResponse} CreateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkGroupResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.CreateBookmarkGroupResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.group = $root.petajournal.BookmarkGroup.decode(reader, reader.uint32());
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
         * Decodes a CreateBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.CreateBookmarkGroupResponse} CreateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkGroupResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a CreateBookmarkGroupResponse message.
         * @function verify
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        CreateBookmarkGroupResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.group != null && message.hasOwnProperty("group")) {
                let error = $root.petajournal.BookmarkGroup.verify(message.group);
                if (error)
                    return "group." + error;
            }
            return null;
        };

        /**
         * Creates a CreateBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.CreateBookmarkGroupResponse} CreateBookmarkGroupResponse
         */
        CreateBookmarkGroupResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.CreateBookmarkGroupResponse)
                return object;
            let message = new $root.petajournal.CreateBookmarkGroupResponse();
            if (object.group != null) {
                if (typeof object.group !== "object")
                    throw TypeError(".petajournal.CreateBookmarkGroupResponse.group: object expected");
                message.group = $root.petajournal.BookmarkGroup.fromObject(object.group);
            }
            return message;
        };

        /**
         * Creates a plain object from a CreateBookmarkGroupResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {petajournal.CreateBookmarkGroupResponse} message CreateBookmarkGroupResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        CreateBookmarkGroupResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.group = null;
            if (message.group != null && message.hasOwnProperty("group"))
                object.group = $root.petajournal.BookmarkGroup.toObject(message.group, options);
            return object;
        };

        /**
         * Converts this CreateBookmarkGroupResponse to JSON.
         * @function toJSON
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        CreateBookmarkGroupResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for CreateBookmarkGroupResponse
         * @function getTypeUrl
         * @memberof petajournal.CreateBookmarkGroupResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        CreateBookmarkGroupResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.CreateBookmarkGroupResponse";
        };

        return CreateBookmarkGroupResponse;
    })();

    petajournal.CreateBookmarkRequest = (function() {

        /**
         * Properties of a CreateBookmarkRequest.
         * @memberof petajournal
         * @interface ICreateBookmarkRequest
         * @property {string|null} [groupId] CreateBookmarkRequest groupId
         * @property {string|null} [targetType] CreateBookmarkRequest targetType
         * @property {string|null} [targetId] CreateBookmarkRequest targetId
         */

        /**
         * Constructs a new CreateBookmarkRequest.
         * @memberof petajournal
         * @classdesc Represents a CreateBookmarkRequest.
         * @implements ICreateBookmarkRequest
         * @constructor
         * @param {petajournal.ICreateBookmarkRequest=} [properties] Properties to set
         */
        function CreateBookmarkRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * CreateBookmarkRequest groupId.
         * @member {string} groupId
         * @memberof petajournal.CreateBookmarkRequest
         * @instance
         */
        CreateBookmarkRequest.prototype.groupId = "";

        /**
         * CreateBookmarkRequest targetType.
         * @member {string} targetType
         * @memberof petajournal.CreateBookmarkRequest
         * @instance
         */
        CreateBookmarkRequest.prototype.targetType = "";

        /**
         * CreateBookmarkRequest targetId.
         * @member {string} targetId
         * @memberof petajournal.CreateBookmarkRequest
         * @instance
         */
        CreateBookmarkRequest.prototype.targetId = "";

        /**
         * Creates a new CreateBookmarkRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {petajournal.ICreateBookmarkRequest=} [properties] Properties to set
         * @returns {petajournal.CreateBookmarkRequest} CreateBookmarkRequest instance
         */
        CreateBookmarkRequest.create = function create(properties) {
            return new CreateBookmarkRequest(properties);
        };

        /**
         * Encodes the specified CreateBookmarkRequest message. Does not implicitly {@link petajournal.CreateBookmarkRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {petajournal.ICreateBookmarkRequest} message CreateBookmarkRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.groupId != null && Object.hasOwnProperty.call(message, "groupId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.groupId);
            if (message.targetType != null && Object.hasOwnProperty.call(message, "targetType"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.targetType);
            if (message.targetId != null && Object.hasOwnProperty.call(message, "targetId"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.targetId);
            return writer;
        };

        /**
         * Encodes the specified CreateBookmarkRequest message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {petajournal.ICreateBookmarkRequest} message CreateBookmarkRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a CreateBookmarkRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.CreateBookmarkRequest} CreateBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.CreateBookmarkRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.groupId = reader.string();
                        break;
                    }
                case 2: {
                        message.targetType = reader.string();
                        break;
                    }
                case 3: {
                        message.targetId = reader.string();
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
         * Decodes a CreateBookmarkRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.CreateBookmarkRequest} CreateBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a CreateBookmarkRequest message.
         * @function verify
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        CreateBookmarkRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                if (!$util.isString(message.groupId))
                    return "groupId: string expected";
            if (message.targetType != null && message.hasOwnProperty("targetType"))
                if (!$util.isString(message.targetType))
                    return "targetType: string expected";
            if (message.targetId != null && message.hasOwnProperty("targetId"))
                if (!$util.isString(message.targetId))
                    return "targetId: string expected";
            return null;
        };

        /**
         * Creates a CreateBookmarkRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.CreateBookmarkRequest} CreateBookmarkRequest
         */
        CreateBookmarkRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.CreateBookmarkRequest)
                return object;
            let message = new $root.petajournal.CreateBookmarkRequest();
            if (object.groupId != null)
                message.groupId = String(object.groupId);
            if (object.targetType != null)
                message.targetType = String(object.targetType);
            if (object.targetId != null)
                message.targetId = String(object.targetId);
            return message;
        };

        /**
         * Creates a plain object from a CreateBookmarkRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {petajournal.CreateBookmarkRequest} message CreateBookmarkRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        CreateBookmarkRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.groupId = "";
                object.targetType = "";
                object.targetId = "";
            }
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                object.groupId = message.groupId;
            if (message.targetType != null && message.hasOwnProperty("targetType"))
                object.targetType = message.targetType;
            if (message.targetId != null && message.hasOwnProperty("targetId"))
                object.targetId = message.targetId;
            return object;
        };

        /**
         * Converts this CreateBookmarkRequest to JSON.
         * @function toJSON
         * @memberof petajournal.CreateBookmarkRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        CreateBookmarkRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for CreateBookmarkRequest
         * @function getTypeUrl
         * @memberof petajournal.CreateBookmarkRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        CreateBookmarkRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.CreateBookmarkRequest";
        };

        return CreateBookmarkRequest;
    })();

    petajournal.CreateBookmarkResponse = (function() {

        /**
         * Properties of a CreateBookmarkResponse.
         * @memberof petajournal
         * @interface ICreateBookmarkResponse
         * @property {petajournal.IBookmark|null} [bookmark] CreateBookmarkResponse bookmark
         */

        /**
         * Constructs a new CreateBookmarkResponse.
         * @memberof petajournal
         * @classdesc Represents a CreateBookmarkResponse.
         * @implements ICreateBookmarkResponse
         * @constructor
         * @param {petajournal.ICreateBookmarkResponse=} [properties] Properties to set
         */
        function CreateBookmarkResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * CreateBookmarkResponse bookmark.
         * @member {petajournal.IBookmark|null|undefined} bookmark
         * @memberof petajournal.CreateBookmarkResponse
         * @instance
         */
        CreateBookmarkResponse.prototype.bookmark = null;

        /**
         * Creates a new CreateBookmarkResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {petajournal.ICreateBookmarkResponse=} [properties] Properties to set
         * @returns {petajournal.CreateBookmarkResponse} CreateBookmarkResponse instance
         */
        CreateBookmarkResponse.create = function create(properties) {
            return new CreateBookmarkResponse(properties);
        };

        /**
         * Encodes the specified CreateBookmarkResponse message. Does not implicitly {@link petajournal.CreateBookmarkResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {petajournal.ICreateBookmarkResponse} message CreateBookmarkResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.bookmark != null && Object.hasOwnProperty.call(message, "bookmark"))
                $root.petajournal.Bookmark.encode(message.bookmark, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified CreateBookmarkResponse message, length delimited. Does not implicitly {@link petajournal.CreateBookmarkResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {petajournal.ICreateBookmarkResponse} message CreateBookmarkResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        CreateBookmarkResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a CreateBookmarkResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.CreateBookmarkResponse} CreateBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.CreateBookmarkResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.bookmark = $root.petajournal.Bookmark.decode(reader, reader.uint32());
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
         * Decodes a CreateBookmarkResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.CreateBookmarkResponse} CreateBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        CreateBookmarkResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a CreateBookmarkResponse message.
         * @function verify
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        CreateBookmarkResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.bookmark != null && message.hasOwnProperty("bookmark")) {
                let error = $root.petajournal.Bookmark.verify(message.bookmark);
                if (error)
                    return "bookmark." + error;
            }
            return null;
        };

        /**
         * Creates a CreateBookmarkResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.CreateBookmarkResponse} CreateBookmarkResponse
         */
        CreateBookmarkResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.CreateBookmarkResponse)
                return object;
            let message = new $root.petajournal.CreateBookmarkResponse();
            if (object.bookmark != null) {
                if (typeof object.bookmark !== "object")
                    throw TypeError(".petajournal.CreateBookmarkResponse.bookmark: object expected");
                message.bookmark = $root.petajournal.Bookmark.fromObject(object.bookmark);
            }
            return message;
        };

        /**
         * Creates a plain object from a CreateBookmarkResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {petajournal.CreateBookmarkResponse} message CreateBookmarkResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        CreateBookmarkResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.bookmark = null;
            if (message.bookmark != null && message.hasOwnProperty("bookmark"))
                object.bookmark = $root.petajournal.Bookmark.toObject(message.bookmark, options);
            return object;
        };

        /**
         * Converts this CreateBookmarkResponse to JSON.
         * @function toJSON
         * @memberof petajournal.CreateBookmarkResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        CreateBookmarkResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for CreateBookmarkResponse
         * @function getTypeUrl
         * @memberof petajournal.CreateBookmarkResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        CreateBookmarkResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.CreateBookmarkResponse";
        };

        return CreateBookmarkResponse;
    })();

    petajournal.DeleteBookmarkRequest = (function() {

        /**
         * Properties of a DeleteBookmarkRequest.
         * @memberof petajournal
         * @interface IDeleteBookmarkRequest
         * @property {string|null} [bookmarkId] DeleteBookmarkRequest bookmarkId
         */

        /**
         * Constructs a new DeleteBookmarkRequest.
         * @memberof petajournal
         * @classdesc Represents a DeleteBookmarkRequest.
         * @implements IDeleteBookmarkRequest
         * @constructor
         * @param {petajournal.IDeleteBookmarkRequest=} [properties] Properties to set
         */
        function DeleteBookmarkRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * DeleteBookmarkRequest bookmarkId.
         * @member {string} bookmarkId
         * @memberof petajournal.DeleteBookmarkRequest
         * @instance
         */
        DeleteBookmarkRequest.prototype.bookmarkId = "";

        /**
         * Creates a new DeleteBookmarkRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {petajournal.IDeleteBookmarkRequest=} [properties] Properties to set
         * @returns {petajournal.DeleteBookmarkRequest} DeleteBookmarkRequest instance
         */
        DeleteBookmarkRequest.create = function create(properties) {
            return new DeleteBookmarkRequest(properties);
        };

        /**
         * Encodes the specified DeleteBookmarkRequest message. Does not implicitly {@link petajournal.DeleteBookmarkRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {petajournal.IDeleteBookmarkRequest} message DeleteBookmarkRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.bookmarkId != null && Object.hasOwnProperty.call(message, "bookmarkId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.bookmarkId);
            return writer;
        };

        /**
         * Encodes the specified DeleteBookmarkRequest message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {petajournal.IDeleteBookmarkRequest} message DeleteBookmarkRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a DeleteBookmarkRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeleteBookmarkRequest} DeleteBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeleteBookmarkRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.bookmarkId = reader.string();
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
         * Decodes a DeleteBookmarkRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeleteBookmarkRequest} DeleteBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeleteBookmarkRequest message.
         * @function verify
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeleteBookmarkRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.bookmarkId != null && message.hasOwnProperty("bookmarkId"))
                if (!$util.isString(message.bookmarkId))
                    return "bookmarkId: string expected";
            return null;
        };

        /**
         * Creates a DeleteBookmarkRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeleteBookmarkRequest} DeleteBookmarkRequest
         */
        DeleteBookmarkRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeleteBookmarkRequest)
                return object;
            let message = new $root.petajournal.DeleteBookmarkRequest();
            if (object.bookmarkId != null)
                message.bookmarkId = String(object.bookmarkId);
            return message;
        };

        /**
         * Creates a plain object from a DeleteBookmarkRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {petajournal.DeleteBookmarkRequest} message DeleteBookmarkRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeleteBookmarkRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.bookmarkId = "";
            if (message.bookmarkId != null && message.hasOwnProperty("bookmarkId"))
                object.bookmarkId = message.bookmarkId;
            return object;
        };

        /**
         * Converts this DeleteBookmarkRequest to JSON.
         * @function toJSON
         * @memberof petajournal.DeleteBookmarkRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeleteBookmarkRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for DeleteBookmarkRequest
         * @function getTypeUrl
         * @memberof petajournal.DeleteBookmarkRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeleteBookmarkRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeleteBookmarkRequest";
        };

        return DeleteBookmarkRequest;
    })();

    petajournal.DeleteBookmarkResponse = (function() {

        /**
         * Properties of a DeleteBookmarkResponse.
         * @memberof petajournal
         * @interface IDeleteBookmarkResponse
         */

        /**
         * Constructs a new DeleteBookmarkResponse.
         * @memberof petajournal
         * @classdesc Represents a DeleteBookmarkResponse.
         * @implements IDeleteBookmarkResponse
         * @constructor
         * @param {petajournal.IDeleteBookmarkResponse=} [properties] Properties to set
         */
        function DeleteBookmarkResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Creates a new DeleteBookmarkResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {petajournal.IDeleteBookmarkResponse=} [properties] Properties to set
         * @returns {petajournal.DeleteBookmarkResponse} DeleteBookmarkResponse instance
         */
        DeleteBookmarkResponse.create = function create(properties) {
            return new DeleteBookmarkResponse(properties);
        };

        /**
         * Encodes the specified DeleteBookmarkResponse message. Does not implicitly {@link petajournal.DeleteBookmarkResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {petajournal.IDeleteBookmarkResponse} message DeleteBookmarkResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            return writer;
        };

        /**
         * Encodes the specified DeleteBookmarkResponse message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {petajournal.IDeleteBookmarkResponse} message DeleteBookmarkResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a DeleteBookmarkResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeleteBookmarkResponse} DeleteBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeleteBookmarkResponse();
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
         * Decodes a DeleteBookmarkResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeleteBookmarkResponse} DeleteBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeleteBookmarkResponse message.
         * @function verify
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeleteBookmarkResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            return null;
        };

        /**
         * Creates a DeleteBookmarkResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeleteBookmarkResponse} DeleteBookmarkResponse
         */
        DeleteBookmarkResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeleteBookmarkResponse)
                return object;
            return new $root.petajournal.DeleteBookmarkResponse();
        };

        /**
         * Creates a plain object from a DeleteBookmarkResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {petajournal.DeleteBookmarkResponse} message DeleteBookmarkResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeleteBookmarkResponse.toObject = function toObject() {
            return {};
        };

        /**
         * Converts this DeleteBookmarkResponse to JSON.
         * @function toJSON
         * @memberof petajournal.DeleteBookmarkResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeleteBookmarkResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for DeleteBookmarkResponse
         * @function getTypeUrl
         * @memberof petajournal.DeleteBookmarkResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeleteBookmarkResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeleteBookmarkResponse";
        };

        return DeleteBookmarkResponse;
    })();

    petajournal.DeleteBookmarkGroupRequest = (function() {

        /**
         * Properties of a DeleteBookmarkGroupRequest.
         * @memberof petajournal
         * @interface IDeleteBookmarkGroupRequest
         * @property {string|null} [groupId] DeleteBookmarkGroupRequest groupId
         */

        /**
         * Constructs a new DeleteBookmarkGroupRequest.
         * @memberof petajournal
         * @classdesc Represents a DeleteBookmarkGroupRequest.
         * @implements IDeleteBookmarkGroupRequest
         * @constructor
         * @param {petajournal.IDeleteBookmarkGroupRequest=} [properties] Properties to set
         */
        function DeleteBookmarkGroupRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * DeleteBookmarkGroupRequest groupId.
         * @member {string} groupId
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @instance
         */
        DeleteBookmarkGroupRequest.prototype.groupId = "";

        /**
         * Creates a new DeleteBookmarkGroupRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {petajournal.IDeleteBookmarkGroupRequest=} [properties] Properties to set
         * @returns {petajournal.DeleteBookmarkGroupRequest} DeleteBookmarkGroupRequest instance
         */
        DeleteBookmarkGroupRequest.create = function create(properties) {
            return new DeleteBookmarkGroupRequest(properties);
        };

        /**
         * Encodes the specified DeleteBookmarkGroupRequest message. Does not implicitly {@link petajournal.DeleteBookmarkGroupRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {petajournal.IDeleteBookmarkGroupRequest} message DeleteBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkGroupRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.groupId != null && Object.hasOwnProperty.call(message, "groupId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.groupId);
            return writer;
        };

        /**
         * Encodes the specified DeleteBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkGroupRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {petajournal.IDeleteBookmarkGroupRequest} message DeleteBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkGroupRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a DeleteBookmarkGroupRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeleteBookmarkGroupRequest} DeleteBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkGroupRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeleteBookmarkGroupRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.groupId = reader.string();
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
         * Decodes a DeleteBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeleteBookmarkGroupRequest} DeleteBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkGroupRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeleteBookmarkGroupRequest message.
         * @function verify
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeleteBookmarkGroupRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                if (!$util.isString(message.groupId))
                    return "groupId: string expected";
            return null;
        };

        /**
         * Creates a DeleteBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeleteBookmarkGroupRequest} DeleteBookmarkGroupRequest
         */
        DeleteBookmarkGroupRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeleteBookmarkGroupRequest)
                return object;
            let message = new $root.petajournal.DeleteBookmarkGroupRequest();
            if (object.groupId != null)
                message.groupId = String(object.groupId);
            return message;
        };

        /**
         * Creates a plain object from a DeleteBookmarkGroupRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {petajournal.DeleteBookmarkGroupRequest} message DeleteBookmarkGroupRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeleteBookmarkGroupRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.groupId = "";
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                object.groupId = message.groupId;
            return object;
        };

        /**
         * Converts this DeleteBookmarkGroupRequest to JSON.
         * @function toJSON
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeleteBookmarkGroupRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for DeleteBookmarkGroupRequest
         * @function getTypeUrl
         * @memberof petajournal.DeleteBookmarkGroupRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeleteBookmarkGroupRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeleteBookmarkGroupRequest";
        };

        return DeleteBookmarkGroupRequest;
    })();

    petajournal.DeleteBookmarkGroupResponse = (function() {

        /**
         * Properties of a DeleteBookmarkGroupResponse.
         * @memberof petajournal
         * @interface IDeleteBookmarkGroupResponse
         */

        /**
         * Constructs a new DeleteBookmarkGroupResponse.
         * @memberof petajournal
         * @classdesc Represents a DeleteBookmarkGroupResponse.
         * @implements IDeleteBookmarkGroupResponse
         * @constructor
         * @param {petajournal.IDeleteBookmarkGroupResponse=} [properties] Properties to set
         */
        function DeleteBookmarkGroupResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Creates a new DeleteBookmarkGroupResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {petajournal.IDeleteBookmarkGroupResponse=} [properties] Properties to set
         * @returns {petajournal.DeleteBookmarkGroupResponse} DeleteBookmarkGroupResponse instance
         */
        DeleteBookmarkGroupResponse.create = function create(properties) {
            return new DeleteBookmarkGroupResponse(properties);
        };

        /**
         * Encodes the specified DeleteBookmarkGroupResponse message. Does not implicitly {@link petajournal.DeleteBookmarkGroupResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {petajournal.IDeleteBookmarkGroupResponse} message DeleteBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkGroupResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            return writer;
        };

        /**
         * Encodes the specified DeleteBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.DeleteBookmarkGroupResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {petajournal.IDeleteBookmarkGroupResponse} message DeleteBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DeleteBookmarkGroupResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a DeleteBookmarkGroupResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.DeleteBookmarkGroupResponse} DeleteBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkGroupResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.DeleteBookmarkGroupResponse();
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
         * Decodes a DeleteBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.DeleteBookmarkGroupResponse} DeleteBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DeleteBookmarkGroupResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DeleteBookmarkGroupResponse message.
         * @function verify
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DeleteBookmarkGroupResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            return null;
        };

        /**
         * Creates a DeleteBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.DeleteBookmarkGroupResponse} DeleteBookmarkGroupResponse
         */
        DeleteBookmarkGroupResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.DeleteBookmarkGroupResponse)
                return object;
            return new $root.petajournal.DeleteBookmarkGroupResponse();
        };

        /**
         * Creates a plain object from a DeleteBookmarkGroupResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {petajournal.DeleteBookmarkGroupResponse} message DeleteBookmarkGroupResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DeleteBookmarkGroupResponse.toObject = function toObject() {
            return {};
        };

        /**
         * Converts this DeleteBookmarkGroupResponse to JSON.
         * @function toJSON
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DeleteBookmarkGroupResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for DeleteBookmarkGroupResponse
         * @function getTypeUrl
         * @memberof petajournal.DeleteBookmarkGroupResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DeleteBookmarkGroupResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.DeleteBookmarkGroupResponse";
        };

        return DeleteBookmarkGroupResponse;
    })();

    petajournal.MoveBookmarkRequest = (function() {

        /**
         * Properties of a MoveBookmarkRequest.
         * @memberof petajournal
         * @interface IMoveBookmarkRequest
         * @property {string|null} [bookmarkId] MoveBookmarkRequest bookmarkId
         * @property {string|null} [targetGroupId] MoveBookmarkRequest targetGroupId
         * @property {string|null} [afterBookmarkId] MoveBookmarkRequest afterBookmarkId
         */

        /**
         * Constructs a new MoveBookmarkRequest.
         * @memberof petajournal
         * @classdesc Represents a MoveBookmarkRequest.
         * @implements IMoveBookmarkRequest
         * @constructor
         * @param {petajournal.IMoveBookmarkRequest=} [properties] Properties to set
         */
        function MoveBookmarkRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * MoveBookmarkRequest bookmarkId.
         * @member {string} bookmarkId
         * @memberof petajournal.MoveBookmarkRequest
         * @instance
         */
        MoveBookmarkRequest.prototype.bookmarkId = "";

        /**
         * MoveBookmarkRequest targetGroupId.
         * @member {string} targetGroupId
         * @memberof petajournal.MoveBookmarkRequest
         * @instance
         */
        MoveBookmarkRequest.prototype.targetGroupId = "";

        /**
         * MoveBookmarkRequest afterBookmarkId.
         * @member {string} afterBookmarkId
         * @memberof petajournal.MoveBookmarkRequest
         * @instance
         */
        MoveBookmarkRequest.prototype.afterBookmarkId = "";

        /**
         * Creates a new MoveBookmarkRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {petajournal.IMoveBookmarkRequest=} [properties] Properties to set
         * @returns {petajournal.MoveBookmarkRequest} MoveBookmarkRequest instance
         */
        MoveBookmarkRequest.create = function create(properties) {
            return new MoveBookmarkRequest(properties);
        };

        /**
         * Encodes the specified MoveBookmarkRequest message. Does not implicitly {@link petajournal.MoveBookmarkRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {petajournal.IMoveBookmarkRequest} message MoveBookmarkRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.bookmarkId != null && Object.hasOwnProperty.call(message, "bookmarkId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.bookmarkId);
            if (message.targetGroupId != null && Object.hasOwnProperty.call(message, "targetGroupId"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.targetGroupId);
            if (message.afterBookmarkId != null && Object.hasOwnProperty.call(message, "afterBookmarkId"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.afterBookmarkId);
            return writer;
        };

        /**
         * Encodes the specified MoveBookmarkRequest message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {petajournal.IMoveBookmarkRequest} message MoveBookmarkRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a MoveBookmarkRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.MoveBookmarkRequest} MoveBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.MoveBookmarkRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.bookmarkId = reader.string();
                        break;
                    }
                case 2: {
                        message.targetGroupId = reader.string();
                        break;
                    }
                case 3: {
                        message.afterBookmarkId = reader.string();
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
         * Decodes a MoveBookmarkRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.MoveBookmarkRequest} MoveBookmarkRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a MoveBookmarkRequest message.
         * @function verify
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        MoveBookmarkRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.bookmarkId != null && message.hasOwnProperty("bookmarkId"))
                if (!$util.isString(message.bookmarkId))
                    return "bookmarkId: string expected";
            if (message.targetGroupId != null && message.hasOwnProperty("targetGroupId"))
                if (!$util.isString(message.targetGroupId))
                    return "targetGroupId: string expected";
            if (message.afterBookmarkId != null && message.hasOwnProperty("afterBookmarkId"))
                if (!$util.isString(message.afterBookmarkId))
                    return "afterBookmarkId: string expected";
            return null;
        };

        /**
         * Creates a MoveBookmarkRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.MoveBookmarkRequest} MoveBookmarkRequest
         */
        MoveBookmarkRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.MoveBookmarkRequest)
                return object;
            let message = new $root.petajournal.MoveBookmarkRequest();
            if (object.bookmarkId != null)
                message.bookmarkId = String(object.bookmarkId);
            if (object.targetGroupId != null)
                message.targetGroupId = String(object.targetGroupId);
            if (object.afterBookmarkId != null)
                message.afterBookmarkId = String(object.afterBookmarkId);
            return message;
        };

        /**
         * Creates a plain object from a MoveBookmarkRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {petajournal.MoveBookmarkRequest} message MoveBookmarkRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        MoveBookmarkRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.bookmarkId = "";
                object.targetGroupId = "";
                object.afterBookmarkId = "";
            }
            if (message.bookmarkId != null && message.hasOwnProperty("bookmarkId"))
                object.bookmarkId = message.bookmarkId;
            if (message.targetGroupId != null && message.hasOwnProperty("targetGroupId"))
                object.targetGroupId = message.targetGroupId;
            if (message.afterBookmarkId != null && message.hasOwnProperty("afterBookmarkId"))
                object.afterBookmarkId = message.afterBookmarkId;
            return object;
        };

        /**
         * Converts this MoveBookmarkRequest to JSON.
         * @function toJSON
         * @memberof petajournal.MoveBookmarkRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        MoveBookmarkRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for MoveBookmarkRequest
         * @function getTypeUrl
         * @memberof petajournal.MoveBookmarkRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        MoveBookmarkRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.MoveBookmarkRequest";
        };

        return MoveBookmarkRequest;
    })();

    petajournal.MoveBookmarkResponse = (function() {

        /**
         * Properties of a MoveBookmarkResponse.
         * @memberof petajournal
         * @interface IMoveBookmarkResponse
         * @property {petajournal.IBookmark|null} [bookmark] MoveBookmarkResponse bookmark
         */

        /**
         * Constructs a new MoveBookmarkResponse.
         * @memberof petajournal
         * @classdesc Represents a MoveBookmarkResponse.
         * @implements IMoveBookmarkResponse
         * @constructor
         * @param {petajournal.IMoveBookmarkResponse=} [properties] Properties to set
         */
        function MoveBookmarkResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * MoveBookmarkResponse bookmark.
         * @member {petajournal.IBookmark|null|undefined} bookmark
         * @memberof petajournal.MoveBookmarkResponse
         * @instance
         */
        MoveBookmarkResponse.prototype.bookmark = null;

        /**
         * Creates a new MoveBookmarkResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {petajournal.IMoveBookmarkResponse=} [properties] Properties to set
         * @returns {petajournal.MoveBookmarkResponse} MoveBookmarkResponse instance
         */
        MoveBookmarkResponse.create = function create(properties) {
            return new MoveBookmarkResponse(properties);
        };

        /**
         * Encodes the specified MoveBookmarkResponse message. Does not implicitly {@link petajournal.MoveBookmarkResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {petajournal.IMoveBookmarkResponse} message MoveBookmarkResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.bookmark != null && Object.hasOwnProperty.call(message, "bookmark"))
                $root.petajournal.Bookmark.encode(message.bookmark, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified MoveBookmarkResponse message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {petajournal.IMoveBookmarkResponse} message MoveBookmarkResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a MoveBookmarkResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.MoveBookmarkResponse} MoveBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.MoveBookmarkResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.bookmark = $root.petajournal.Bookmark.decode(reader, reader.uint32());
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
         * Decodes a MoveBookmarkResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.MoveBookmarkResponse} MoveBookmarkResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a MoveBookmarkResponse message.
         * @function verify
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        MoveBookmarkResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.bookmark != null && message.hasOwnProperty("bookmark")) {
                let error = $root.petajournal.Bookmark.verify(message.bookmark);
                if (error)
                    return "bookmark." + error;
            }
            return null;
        };

        /**
         * Creates a MoveBookmarkResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.MoveBookmarkResponse} MoveBookmarkResponse
         */
        MoveBookmarkResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.MoveBookmarkResponse)
                return object;
            let message = new $root.petajournal.MoveBookmarkResponse();
            if (object.bookmark != null) {
                if (typeof object.bookmark !== "object")
                    throw TypeError(".petajournal.MoveBookmarkResponse.bookmark: object expected");
                message.bookmark = $root.petajournal.Bookmark.fromObject(object.bookmark);
            }
            return message;
        };

        /**
         * Creates a plain object from a MoveBookmarkResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {petajournal.MoveBookmarkResponse} message MoveBookmarkResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        MoveBookmarkResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.bookmark = null;
            if (message.bookmark != null && message.hasOwnProperty("bookmark"))
                object.bookmark = $root.petajournal.Bookmark.toObject(message.bookmark, options);
            return object;
        };

        /**
         * Converts this MoveBookmarkResponse to JSON.
         * @function toJSON
         * @memberof petajournal.MoveBookmarkResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        MoveBookmarkResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for MoveBookmarkResponse
         * @function getTypeUrl
         * @memberof petajournal.MoveBookmarkResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        MoveBookmarkResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.MoveBookmarkResponse";
        };

        return MoveBookmarkResponse;
    })();

    petajournal.MoveBookmarkGroupRequest = (function() {

        /**
         * Properties of a MoveBookmarkGroupRequest.
         * @memberof petajournal
         * @interface IMoveBookmarkGroupRequest
         * @property {string|null} [groupId] MoveBookmarkGroupRequest groupId
         * @property {string|null} [afterGroupId] MoveBookmarkGroupRequest afterGroupId
         */

        /**
         * Constructs a new MoveBookmarkGroupRequest.
         * @memberof petajournal
         * @classdesc Represents a MoveBookmarkGroupRequest.
         * @implements IMoveBookmarkGroupRequest
         * @constructor
         * @param {petajournal.IMoveBookmarkGroupRequest=} [properties] Properties to set
         */
        function MoveBookmarkGroupRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * MoveBookmarkGroupRequest groupId.
         * @member {string} groupId
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @instance
         */
        MoveBookmarkGroupRequest.prototype.groupId = "";

        /**
         * MoveBookmarkGroupRequest afterGroupId.
         * @member {string} afterGroupId
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @instance
         */
        MoveBookmarkGroupRequest.prototype.afterGroupId = "";

        /**
         * Creates a new MoveBookmarkGroupRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {petajournal.IMoveBookmarkGroupRequest=} [properties] Properties to set
         * @returns {petajournal.MoveBookmarkGroupRequest} MoveBookmarkGroupRequest instance
         */
        MoveBookmarkGroupRequest.create = function create(properties) {
            return new MoveBookmarkGroupRequest(properties);
        };

        /**
         * Encodes the specified MoveBookmarkGroupRequest message. Does not implicitly {@link petajournal.MoveBookmarkGroupRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {petajournal.IMoveBookmarkGroupRequest} message MoveBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkGroupRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.groupId != null && Object.hasOwnProperty.call(message, "groupId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.groupId);
            if (message.afterGroupId != null && Object.hasOwnProperty.call(message, "afterGroupId"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.afterGroupId);
            return writer;
        };

        /**
         * Encodes the specified MoveBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkGroupRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {petajournal.IMoveBookmarkGroupRequest} message MoveBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkGroupRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a MoveBookmarkGroupRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.MoveBookmarkGroupRequest} MoveBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkGroupRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.MoveBookmarkGroupRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.groupId = reader.string();
                        break;
                    }
                case 2: {
                        message.afterGroupId = reader.string();
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
         * Decodes a MoveBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.MoveBookmarkGroupRequest} MoveBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkGroupRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a MoveBookmarkGroupRequest message.
         * @function verify
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        MoveBookmarkGroupRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                if (!$util.isString(message.groupId))
                    return "groupId: string expected";
            if (message.afterGroupId != null && message.hasOwnProperty("afterGroupId"))
                if (!$util.isString(message.afterGroupId))
                    return "afterGroupId: string expected";
            return null;
        };

        /**
         * Creates a MoveBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.MoveBookmarkGroupRequest} MoveBookmarkGroupRequest
         */
        MoveBookmarkGroupRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.MoveBookmarkGroupRequest)
                return object;
            let message = new $root.petajournal.MoveBookmarkGroupRequest();
            if (object.groupId != null)
                message.groupId = String(object.groupId);
            if (object.afterGroupId != null)
                message.afterGroupId = String(object.afterGroupId);
            return message;
        };

        /**
         * Creates a plain object from a MoveBookmarkGroupRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {petajournal.MoveBookmarkGroupRequest} message MoveBookmarkGroupRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        MoveBookmarkGroupRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.groupId = "";
                object.afterGroupId = "";
            }
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                object.groupId = message.groupId;
            if (message.afterGroupId != null && message.hasOwnProperty("afterGroupId"))
                object.afterGroupId = message.afterGroupId;
            return object;
        };

        /**
         * Converts this MoveBookmarkGroupRequest to JSON.
         * @function toJSON
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        MoveBookmarkGroupRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for MoveBookmarkGroupRequest
         * @function getTypeUrl
         * @memberof petajournal.MoveBookmarkGroupRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        MoveBookmarkGroupRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.MoveBookmarkGroupRequest";
        };

        return MoveBookmarkGroupRequest;
    })();

    petajournal.MoveBookmarkGroupResponse = (function() {

        /**
         * Properties of a MoveBookmarkGroupResponse.
         * @memberof petajournal
         * @interface IMoveBookmarkGroupResponse
         * @property {petajournal.IBookmarkGroup|null} [group] MoveBookmarkGroupResponse group
         */

        /**
         * Constructs a new MoveBookmarkGroupResponse.
         * @memberof petajournal
         * @classdesc Represents a MoveBookmarkGroupResponse.
         * @implements IMoveBookmarkGroupResponse
         * @constructor
         * @param {petajournal.IMoveBookmarkGroupResponse=} [properties] Properties to set
         */
        function MoveBookmarkGroupResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * MoveBookmarkGroupResponse group.
         * @member {petajournal.IBookmarkGroup|null|undefined} group
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @instance
         */
        MoveBookmarkGroupResponse.prototype.group = null;

        /**
         * Creates a new MoveBookmarkGroupResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {petajournal.IMoveBookmarkGroupResponse=} [properties] Properties to set
         * @returns {petajournal.MoveBookmarkGroupResponse} MoveBookmarkGroupResponse instance
         */
        MoveBookmarkGroupResponse.create = function create(properties) {
            return new MoveBookmarkGroupResponse(properties);
        };

        /**
         * Encodes the specified MoveBookmarkGroupResponse message. Does not implicitly {@link petajournal.MoveBookmarkGroupResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {petajournal.IMoveBookmarkGroupResponse} message MoveBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkGroupResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.group != null && Object.hasOwnProperty.call(message, "group"))
                $root.petajournal.BookmarkGroup.encode(message.group, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified MoveBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.MoveBookmarkGroupResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {petajournal.IMoveBookmarkGroupResponse} message MoveBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        MoveBookmarkGroupResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a MoveBookmarkGroupResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.MoveBookmarkGroupResponse} MoveBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkGroupResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.MoveBookmarkGroupResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.group = $root.petajournal.BookmarkGroup.decode(reader, reader.uint32());
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
         * Decodes a MoveBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.MoveBookmarkGroupResponse} MoveBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        MoveBookmarkGroupResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a MoveBookmarkGroupResponse message.
         * @function verify
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        MoveBookmarkGroupResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.group != null && message.hasOwnProperty("group")) {
                let error = $root.petajournal.BookmarkGroup.verify(message.group);
                if (error)
                    return "group." + error;
            }
            return null;
        };

        /**
         * Creates a MoveBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.MoveBookmarkGroupResponse} MoveBookmarkGroupResponse
         */
        MoveBookmarkGroupResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.MoveBookmarkGroupResponse)
                return object;
            let message = new $root.petajournal.MoveBookmarkGroupResponse();
            if (object.group != null) {
                if (typeof object.group !== "object")
                    throw TypeError(".petajournal.MoveBookmarkGroupResponse.group: object expected");
                message.group = $root.petajournal.BookmarkGroup.fromObject(object.group);
            }
            return message;
        };

        /**
         * Creates a plain object from a MoveBookmarkGroupResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {petajournal.MoveBookmarkGroupResponse} message MoveBookmarkGroupResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        MoveBookmarkGroupResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.group = null;
            if (message.group != null && message.hasOwnProperty("group"))
                object.group = $root.petajournal.BookmarkGroup.toObject(message.group, options);
            return object;
        };

        /**
         * Converts this MoveBookmarkGroupResponse to JSON.
         * @function toJSON
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        MoveBookmarkGroupResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for MoveBookmarkGroupResponse
         * @function getTypeUrl
         * @memberof petajournal.MoveBookmarkGroupResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        MoveBookmarkGroupResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.MoveBookmarkGroupResponse";
        };

        return MoveBookmarkGroupResponse;
    })();

    petajournal.UpdateBookmarkGroupRequest = (function() {

        /**
         * Properties of an UpdateBookmarkGroupRequest.
         * @memberof petajournal
         * @interface IUpdateBookmarkGroupRequest
         * @property {string|null} [groupId] UpdateBookmarkGroupRequest groupId
         * @property {string|null} [name] UpdateBookmarkGroupRequest name
         */

        /**
         * Constructs a new UpdateBookmarkGroupRequest.
         * @memberof petajournal
         * @classdesc Represents an UpdateBookmarkGroupRequest.
         * @implements IUpdateBookmarkGroupRequest
         * @constructor
         * @param {petajournal.IUpdateBookmarkGroupRequest=} [properties] Properties to set
         */
        function UpdateBookmarkGroupRequest(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateBookmarkGroupRequest groupId.
         * @member {string} groupId
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @instance
         */
        UpdateBookmarkGroupRequest.prototype.groupId = "";

        /**
         * UpdateBookmarkGroupRequest name.
         * @member {string} name
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @instance
         */
        UpdateBookmarkGroupRequest.prototype.name = "";

        /**
         * Creates a new UpdateBookmarkGroupRequest instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {petajournal.IUpdateBookmarkGroupRequest=} [properties] Properties to set
         * @returns {petajournal.UpdateBookmarkGroupRequest} UpdateBookmarkGroupRequest instance
         */
        UpdateBookmarkGroupRequest.create = function create(properties) {
            return new UpdateBookmarkGroupRequest(properties);
        };

        /**
         * Encodes the specified UpdateBookmarkGroupRequest message. Does not implicitly {@link petajournal.UpdateBookmarkGroupRequest.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {petajournal.IUpdateBookmarkGroupRequest} message UpdateBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBookmarkGroupRequest.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.groupId != null && Object.hasOwnProperty.call(message, "groupId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.groupId);
            if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.name);
            return writer;
        };

        /**
         * Encodes the specified UpdateBookmarkGroupRequest message, length delimited. Does not implicitly {@link petajournal.UpdateBookmarkGroupRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {petajournal.IUpdateBookmarkGroupRequest} message UpdateBookmarkGroupRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBookmarkGroupRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes an UpdateBookmarkGroupRequest message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateBookmarkGroupRequest} UpdateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBookmarkGroupRequest.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateBookmarkGroupRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.groupId = reader.string();
                        break;
                    }
                case 2: {
                        message.name = reader.string();
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
         * Decodes an UpdateBookmarkGroupRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateBookmarkGroupRequest} UpdateBookmarkGroupRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBookmarkGroupRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateBookmarkGroupRequest message.
         * @function verify
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateBookmarkGroupRequest.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                if (!$util.isString(message.groupId))
                    return "groupId: string expected";
            if (message.name != null && message.hasOwnProperty("name"))
                if (!$util.isString(message.name))
                    return "name: string expected";
            return null;
        };

        /**
         * Creates an UpdateBookmarkGroupRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateBookmarkGroupRequest} UpdateBookmarkGroupRequest
         */
        UpdateBookmarkGroupRequest.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateBookmarkGroupRequest)
                return object;
            let message = new $root.petajournal.UpdateBookmarkGroupRequest();
            if (object.groupId != null)
                message.groupId = String(object.groupId);
            if (object.name != null)
                message.name = String(object.name);
            return message;
        };

        /**
         * Creates a plain object from an UpdateBookmarkGroupRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {petajournal.UpdateBookmarkGroupRequest} message UpdateBookmarkGroupRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateBookmarkGroupRequest.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults) {
                object.groupId = "";
                object.name = "";
            }
            if (message.groupId != null && message.hasOwnProperty("groupId"))
                object.groupId = message.groupId;
            if (message.name != null && message.hasOwnProperty("name"))
                object.name = message.name;
            return object;
        };

        /**
         * Converts this UpdateBookmarkGroupRequest to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateBookmarkGroupRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for UpdateBookmarkGroupRequest
         * @function getTypeUrl
         * @memberof petajournal.UpdateBookmarkGroupRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateBookmarkGroupRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateBookmarkGroupRequest";
        };

        return UpdateBookmarkGroupRequest;
    })();

    petajournal.UpdateBookmarkGroupResponse = (function() {

        /**
         * Properties of an UpdateBookmarkGroupResponse.
         * @memberof petajournal
         * @interface IUpdateBookmarkGroupResponse
         * @property {petajournal.IBookmarkGroup|null} [group] UpdateBookmarkGroupResponse group
         */

        /**
         * Constructs a new UpdateBookmarkGroupResponse.
         * @memberof petajournal
         * @classdesc Represents an UpdateBookmarkGroupResponse.
         * @implements IUpdateBookmarkGroupResponse
         * @constructor
         * @param {petajournal.IUpdateBookmarkGroupResponse=} [properties] Properties to set
         */
        function UpdateBookmarkGroupResponse(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * UpdateBookmarkGroupResponse group.
         * @member {petajournal.IBookmarkGroup|null|undefined} group
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @instance
         */
        UpdateBookmarkGroupResponse.prototype.group = null;

        /**
         * Creates a new UpdateBookmarkGroupResponse instance using the specified properties.
         * @function create
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {petajournal.IUpdateBookmarkGroupResponse=} [properties] Properties to set
         * @returns {petajournal.UpdateBookmarkGroupResponse} UpdateBookmarkGroupResponse instance
         */
        UpdateBookmarkGroupResponse.create = function create(properties) {
            return new UpdateBookmarkGroupResponse(properties);
        };

        /**
         * Encodes the specified UpdateBookmarkGroupResponse message. Does not implicitly {@link petajournal.UpdateBookmarkGroupResponse.verify|verify} messages.
         * @function encode
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {petajournal.IUpdateBookmarkGroupResponse} message UpdateBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBookmarkGroupResponse.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.group != null && Object.hasOwnProperty.call(message, "group"))
                $root.petajournal.BookmarkGroup.encode(message.group, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
            return writer;
        };

        /**
         * Encodes the specified UpdateBookmarkGroupResponse message, length delimited. Does not implicitly {@link petajournal.UpdateBookmarkGroupResponse.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {petajournal.IUpdateBookmarkGroupResponse} message UpdateBookmarkGroupResponse message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        UpdateBookmarkGroupResponse.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes an UpdateBookmarkGroupResponse message from the specified reader or buffer.
         * @function decode
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {petajournal.UpdateBookmarkGroupResponse} UpdateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBookmarkGroupResponse.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            let end = length === undefined ? reader.len : reader.pos + length, message = new $root.petajournal.UpdateBookmarkGroupResponse();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.group = $root.petajournal.BookmarkGroup.decode(reader, reader.uint32());
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
         * Decodes an UpdateBookmarkGroupResponse message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.UpdateBookmarkGroupResponse} UpdateBookmarkGroupResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        UpdateBookmarkGroupResponse.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an UpdateBookmarkGroupResponse message.
         * @function verify
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        UpdateBookmarkGroupResponse.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.group != null && message.hasOwnProperty("group")) {
                let error = $root.petajournal.BookmarkGroup.verify(message.group);
                if (error)
                    return "group." + error;
            }
            return null;
        };

        /**
         * Creates an UpdateBookmarkGroupResponse message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.UpdateBookmarkGroupResponse} UpdateBookmarkGroupResponse
         */
        UpdateBookmarkGroupResponse.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.UpdateBookmarkGroupResponse)
                return object;
            let message = new $root.petajournal.UpdateBookmarkGroupResponse();
            if (object.group != null) {
                if (typeof object.group !== "object")
                    throw TypeError(".petajournal.UpdateBookmarkGroupResponse.group: object expected");
                message.group = $root.petajournal.BookmarkGroup.fromObject(object.group);
            }
            return message;
        };

        /**
         * Creates a plain object from an UpdateBookmarkGroupResponse message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {petajournal.UpdateBookmarkGroupResponse} message UpdateBookmarkGroupResponse
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        UpdateBookmarkGroupResponse.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.group = null;
            if (message.group != null && message.hasOwnProperty("group"))
                object.group = $root.petajournal.BookmarkGroup.toObject(message.group, options);
            return object;
        };

        /**
         * Converts this UpdateBookmarkGroupResponse to JSON.
         * @function toJSON
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        UpdateBookmarkGroupResponse.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for UpdateBookmarkGroupResponse
         * @function getTypeUrl
         * @memberof petajournal.UpdateBookmarkGroupResponse
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        UpdateBookmarkGroupResponse.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/petajournal.UpdateBookmarkGroupResponse";
        };

        return UpdateBookmarkGroupResponse;
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
         * Encodes the specified ServerEvent message, length delimited. Does not implicitly {@link petajournal.ServerEvent.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ServerEvent
         * @static
         * @param {petajournal.IServerEvent} message ServerEvent message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ServerEvent.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a ServerEvent message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ServerEvent
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ServerEvent} ServerEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ServerEvent.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ServerEvent message.
         * @function verify
         * @memberof petajournal.ServerEvent
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ServerEvent.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            let properties = {};
            if (message.originatorSessionId != null && message.hasOwnProperty("originatorSessionId"))
                if (!$util.isString(message.originatorSessionId))
                    return "originatorSessionId: string expected";
            if (message.connected != null && message.hasOwnProperty("connected")) {
                properties.event = 1;
                {
                    let error = $root.petajournal.ConnectedEvent.verify(message.connected);
                    if (error)
                        return "connected." + error;
                }
            }
            if (message.pagesUpdated != null && message.hasOwnProperty("pagesUpdated")) {
                if (properties.event === 1)
                    return "event: multiple values";
                properties.event = 1;
                {
                    let error = $root.petajournal.PagesUpdatedEvent.verify(message.pagesUpdated);
                    if (error)
                        return "pagesUpdated." + error;
                }
            }
            return null;
        };

        /**
         * Creates a ServerEvent message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ServerEvent
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ServerEvent} ServerEvent
         */
        ServerEvent.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ServerEvent)
                return object;
            let message = new $root.petajournal.ServerEvent();
            if (object.originatorSessionId != null)
                message.originatorSessionId = String(object.originatorSessionId);
            if (object.connected != null) {
                if (typeof object.connected !== "object")
                    throw TypeError(".petajournal.ServerEvent.connected: object expected");
                message.connected = $root.petajournal.ConnectedEvent.fromObject(object.connected);
            }
            if (object.pagesUpdated != null) {
                if (typeof object.pagesUpdated !== "object")
                    throw TypeError(".petajournal.ServerEvent.pagesUpdated: object expected");
                message.pagesUpdated = $root.petajournal.PagesUpdatedEvent.fromObject(object.pagesUpdated);
            }
            return message;
        };

        /**
         * Creates a plain object from a ServerEvent message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ServerEvent
         * @static
         * @param {petajournal.ServerEvent} message ServerEvent
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ServerEvent.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.originatorSessionId = "";
            if (message.originatorSessionId != null && message.hasOwnProperty("originatorSessionId"))
                object.originatorSessionId = message.originatorSessionId;
            if (message.connected != null && message.hasOwnProperty("connected")) {
                object.connected = $root.petajournal.ConnectedEvent.toObject(message.connected, options);
                if (options.oneofs)
                    object.event = "connected";
            }
            if (message.pagesUpdated != null && message.hasOwnProperty("pagesUpdated")) {
                object.pagesUpdated = $root.petajournal.PagesUpdatedEvent.toObject(message.pagesUpdated, options);
                if (options.oneofs)
                    object.event = "pagesUpdated";
            }
            return object;
        };

        /**
         * Converts this ServerEvent to JSON.
         * @function toJSON
         * @memberof petajournal.ServerEvent
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ServerEvent.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified ConnectedEvent message, length delimited. Does not implicitly {@link petajournal.ConnectedEvent.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {petajournal.IConnectedEvent} message ConnectedEvent message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ConnectedEvent.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a ConnectedEvent message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.ConnectedEvent} ConnectedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ConnectedEvent.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ConnectedEvent message.
         * @function verify
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ConnectedEvent.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.sessionId != null && message.hasOwnProperty("sessionId"))
                if (!$util.isString(message.sessionId))
                    return "sessionId: string expected";
            return null;
        };

        /**
         * Creates a ConnectedEvent message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.ConnectedEvent} ConnectedEvent
         */
        ConnectedEvent.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.ConnectedEvent)
                return object;
            let message = new $root.petajournal.ConnectedEvent();
            if (object.sessionId != null)
                message.sessionId = String(object.sessionId);
            return message;
        };

        /**
         * Creates a plain object from a ConnectedEvent message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.ConnectedEvent
         * @static
         * @param {petajournal.ConnectedEvent} message ConnectedEvent
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ConnectedEvent.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.sessionId = "";
            if (message.sessionId != null && message.hasOwnProperty("sessionId"))
                object.sessionId = message.sessionId;
            return object;
        };

        /**
         * Converts this ConnectedEvent to JSON.
         * @function toJSON
         * @memberof petajournal.ConnectedEvent
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ConnectedEvent.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
         * Encodes the specified PagesUpdatedEvent message, length delimited. Does not implicitly {@link petajournal.PagesUpdatedEvent.verify|verify} messages.
         * @function encodeDelimited
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {petajournal.IPagesUpdatedEvent} message PagesUpdatedEvent message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        PagesUpdatedEvent.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
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
         * Decodes a PagesUpdatedEvent message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {petajournal.PagesUpdatedEvent} PagesUpdatedEvent
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        PagesUpdatedEvent.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a PagesUpdatedEvent message.
         * @function verify
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        PagesUpdatedEvent.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                if (!$util.isString(message.pageId))
                    return "pageId: string expected";
            return null;
        };

        /**
         * Creates a PagesUpdatedEvent message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {petajournal.PagesUpdatedEvent} PagesUpdatedEvent
         */
        PagesUpdatedEvent.fromObject = function fromObject(object) {
            if (object instanceof $root.petajournal.PagesUpdatedEvent)
                return object;
            let message = new $root.petajournal.PagesUpdatedEvent();
            if (object.pageId != null)
                message.pageId = String(object.pageId);
            return message;
        };

        /**
         * Creates a plain object from a PagesUpdatedEvent message. Also converts values to other types if specified.
         * @function toObject
         * @memberof petajournal.PagesUpdatedEvent
         * @static
         * @param {petajournal.PagesUpdatedEvent} message PagesUpdatedEvent
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        PagesUpdatedEvent.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            let object = {};
            if (options.defaults)
                object.pageId = "";
            if (message.pageId != null && message.hasOwnProperty("pageId"))
                object.pageId = message.pageId;
            return object;
        };

        /**
         * Converts this PagesUpdatedEvent to JSON.
         * @function toJSON
         * @memberof petajournal.PagesUpdatedEvent
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        PagesUpdatedEvent.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
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
