CREATE VIRTUAL TABLE content_fts USING fts5(
    title,
    body,
    tags,
    content='',
    content_rowid='rowid'
);

CREATE TRIGGER content_fts_insert AFTER INSERT ON content BEGIN
    INSERT INTO content_fts(rowid, title, body, tags)
    SELECT NEW.id, NEW.title, NEW.body_markdown,
           COALESCE((SELECT GROUP_CONCAT(t.name, ' ') FROM tags t
                     JOIN content_tags ct ON t.id = ct.tag_id
                     WHERE ct.content_id = NEW.id), '');
END;

CREATE TRIGGER content_fts_update AFTER UPDATE ON content BEGIN
    INSERT INTO content_fts(content_fts, rowid, title, body, tags)
    VALUES('delete', OLD.id, OLD.title, OLD.body_markdown,
           COALESCE((SELECT GROUP_CONCAT(t.name, ' ') FROM tags t
                     JOIN content_tags ct ON t.id = ct.tag_id
                     WHERE ct.content_id = OLD.id), ''));
    INSERT INTO content_fts(rowid, title, body, tags)
    SELECT NEW.id, NEW.title, NEW.body_markdown,
           COALESCE((SELECT GROUP_CONCAT(t.name, ' ') FROM tags t
                     JOIN content_tags ct ON t.id = ct.tag_id
                     WHERE ct.content_id = NEW.id), '');
END;

CREATE TRIGGER content_fts_delete AFTER DELETE ON content BEGIN
    INSERT INTO content_fts(content_fts, rowid, title, body, tags)
    VALUES('delete', OLD.id, OLD.title, OLD.body_markdown,
           COALESCE((SELECT GROUP_CONCAT(t.name, ' ') FROM tags t
                     JOIN content_tags ct ON t.id = ct.tag_id
                     WHERE ct.content_id = OLD.id), ''));
END;
