CREATE USER easyvote_usr WITH ENCRYPTED PASSWORD '5ef8ec32dd82';

GRANT CONNECT ON DATABASE db0 TO easyvote_usr;

\connect db0

CREATE SCHEMA easyvote AUTHORIZATION easyvote_usr;

SET search_path = easyvote;
ALTER ROLE easyvote_usr SET search_path = easyvote;

GRANT CREATE ON SCHEMA easyvote TO easyvote_usr;
GRANT USAGE  ON SCHEMA easyvote TO easyvote_usr;
