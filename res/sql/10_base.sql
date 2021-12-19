DROP TABLE IF EXISTS poll CASCADE;
DROP TABLE IF EXISTS poll_option CASCADE;
DROP TABLE IF EXISTS ballot CASCADE;
DROP TABLE IF EXISTS ballot_choice CASCADE;

CREATE TABLE poll
(
    id            INT         NOT NULL GENERATED ALWAYS AS IDENTITY,
    time_created  timestamptz NOT NULL,
    id_server     VARCHAR(20) NOT NULL,
    id_created_by VARCHAR(20) NOT NULL,
    open          bool        NOT NULL,
    name          VARCHAR(24) NOT NULL,
    question      TEXT        NOT NULL,
    ranks         INT         NOT NULL,

    CONSTRAINT poll_pk PRIMARY KEY (id),
    CONSTRAINT poll_id_server_name_uniq UNIQUE (id_server, name)
);

CREATE TABLE poll_option
(
    id_poll INT  NOT NULL,
    id      INT  NOT NULL GENERATED ALWAYS AS IDENTITY,
    option  TEXT NOT NULL,

    CONSTRAINT poll_option_pk PRIMARY KEY (id),
    CONSTRAINT poll_option_uniq UNIQUE (id_poll, option),
    CONSTRAINT poll_option_id_poll_fk FOREIGN KEY (id_poll) REFERENCES poll (id)
);

CREATE TABLE ballot
(
    id           INT         NOT NULL GENERATED ALWAYS AS IDENTITY,
    id_poll      INT         NOT NULL,
    id_user      VARCHAR(20) NOT NULL,
    time_created timestamptz NOT NULL,
    invalidated  bool        NOT NULL,

    CONSTRAINT ballot_pk PRIMARY KEY (id),
    CONSTRAINT ballot_valid_uniq UNIQUE (id_poll, id_user, time_created, invalidated),
    CONSTRAINT ballot_id_poll_fk FOREIGN KEY (id_poll) REFERENCES poll (id)
);

CREATE TABLE ballot_choice
(
    id_ballot INT NOT NULL,
    id_option INT NOT NULL,
    rank      INT NOT NULL,

    CONSTRAINT ballot_choice_pk PRIMARY KEY (id_ballot, id_option),
    CONSTRAINT ballot_choice_ballot_fk FOREIGN KEY (id_ballot) REFERENCES ballot (id),
    CONSTRAINT ballot_choice_option_fk FOREIGN KEY (id_option) REFERENCES poll_option (id)
);
